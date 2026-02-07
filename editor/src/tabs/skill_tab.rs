//! 技能編輯器 tab

use crate::constants::*;
use crate::editor_item::EditorItem;
use board::alias::Coord;
use board::loader_schema::{
    AoeShape, AttackStyle, Attribute, AttributeSource, Mechanic, SaveType, SkillEffect, SkillType,
    TargetFilter, TargetMode, ValueFormula,
};
use strum::IntoEnumIterator;

// ==================== EditorItem 實作 ====================

impl EditorItem for SkillType {
    type UIState = ();

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn type_name() -> &'static str {
        "技能"
    }

    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("名稱不能為空".to_string());
        }
        if self.mp_change > 0 {
            return Err("MP 消耗不能為正數".to_string());
        }
        Ok(())
    }
}

/// 取得技能的檔案名稱
pub fn file_name() -> &'static str {
    "skills"
}

// ==================== 表單渲染 ====================

/// 渲染技能編輯表單
pub fn render_form(ui: &mut egui::Ui, skill: &mut SkillType, _ui_state: &mut ()) {
    ui.horizontal(|ui| {
        ui.label("名稱：");
        ui.text_edit_singleline(&mut skill.name);
    });

    ui.horizontal(|ui| {
        ui.label("MP 消耗：");
        ui.add(
            egui::DragValue::new(&mut skill.mp_change)
                .speed(DRAG_VALUE_SPEED)
                .range(i32::MIN..=0),
        );
    });

    ui.horizontal(|ui| {
        ui.label("最小施放距離：");
        ui.add(
            egui::DragValue::new(&mut skill.min_range)
                .speed(DRAG_VALUE_SPEED)
                .range(0..=Coord::MAX),
        );
    });

    ui.horizontal(|ui| {
        ui.label("最大施放距離：");
        ui.add(
            egui::DragValue::new(&mut skill.max_range)
                .speed(DRAG_VALUE_SPEED)
                .range(0..=Coord::MAX),
        );
    });

    ui.horizontal(|ui| {
        ui.label("使用後可移動：");
        ui.checkbox(&mut skill.allows_movement_after, "");
    });

    ui.label("標籤（以逗號分隔）：");
    let mut tags_text = skill.tags.join(", ");
    ui.text_edit_singleline(&mut tags_text);
    skill.tags = tags_text
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    ui.separator();
    ui.heading("技能效果");

    // 新增效果按鈕
    ui.horizontal(|ui| {
        if ui.button("新增 HP 修正").clicked() {
            skill.effects.push(SkillEffect::HpModify {
                mechanic: Mechanic::HitBased {
                    hit_bonus: 0,
                    crit_rate: 5,
                },
                target_mode: TargetMode::SingleTarget {
                    filter: TargetFilter::All,
                },
                formula: ValueFormula::Fixed { value: 0 },
                style: AttackStyle::Physical,
            });
        }
        if ui.button("新增屬性修正").clicked() {
            skill.effects.push(SkillEffect::AttributeModify {
                mechanic: Mechanic::Guaranteed,
                target_mode: TargetMode::SingleTarget {
                    filter: TargetFilter::Caster,
                },
                formula: ValueFormula::Fixed { value: 0 },
                attribute: Attribute::PhysicalAttack,
                duration: None,
            });
        }
        if ui.button("新增推離").clicked() {
            skill.effects.push(SkillEffect::Push {
                mechanic: Mechanic::DcBased {
                    dc: 0,
                    save_type: SaveType::Fortitude,
                },
                target_mode: TargetMode::SingleTarget {
                    filter: TargetFilter::All,
                },
                distance: 1,
            });
        }
    });

    ui.add_space(5.0);

    // 渲染每個效果
    let mut to_remove = None;
    for (index, effect) in skill.effects.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("效果 #{}", index + 1));
                if ui.button("刪除").clicked() {
                    to_remove = Some(index);
                }
            });
            ui.separator();
            render_effect_form(ui, effect, index);
        });
        ui.add_space(5.0);
    }

    // 刪除標記的效果
    if let Some(index) = to_remove {
        skill.effects.remove(index);
    }
}

/// 渲染判定機制編輯表單
fn render_mechanic_form(ui: &mut egui::Ui, mechanic: &mut Mechanic, salt: &str) {
    let current_type = match mechanic {
        Mechanic::HitBased { .. } => MECHANIC_TYPE_HITBASED,
        Mechanic::DcBased { .. } => MECHANIC_TYPE_DCBASED,
        Mechanic::Guaranteed => MECHANIC_TYPE_GUARANTEED,
    };

    ui.horizontal(|ui| {
        ui.label("判定機制：");
        egui::ComboBox::from_id_salt(salt)
            .selected_text(current_type)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(
                        current_type == MECHANIC_TYPE_HITBASED,
                        MECHANIC_TYPE_HITBASED,
                    )
                    .clicked()
                {
                    *mechanic = Mechanic::HitBased {
                        hit_bonus: 0,
                        crit_rate: 5,
                    };
                }
                if ui
                    .selectable_label(current_type == MECHANIC_TYPE_DCBASED, MECHANIC_TYPE_DCBASED)
                    .clicked()
                {
                    *mechanic = Mechanic::DcBased {
                        dc: 0,
                        save_type: SaveType::Fortitude,
                    };
                }
                if ui
                    .selectable_label(
                        current_type == MECHANIC_TYPE_GUARANTEED,
                        MECHANIC_TYPE_GUARANTEED,
                    )
                    .clicked()
                {
                    *mechanic = Mechanic::Guaranteed;
                }
            });
    });

    match mechanic {
        Mechanic::HitBased {
            hit_bonus,
            crit_rate,
        } => {
            ui.horizontal(|ui| {
                ui.label("命中加值：");
                ui.add(egui::DragValue::new(hit_bonus).speed(DRAG_VALUE_SPEED));
            });
            ui.horizontal(|ui| {
                ui.label("暴擊率：");
                ui.add(
                    egui::DragValue::new(crit_rate)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=100),
                );
            });
        }
        Mechanic::DcBased { dc, save_type } => {
            ui.horizontal(|ui| {
                ui.label("DC：");
                ui.add(
                    egui::DragValue::new(dc)
                        .speed(DRAG_VALUE_SPEED)
                        .range(i32::MIN..=i32::MAX),
                );
            });
            ui.horizontal(|ui| {
                ui.label("檢定類型：");
                egui::ComboBox::from_id_salt(&format!("{}_save_type", salt))
                    .selected_text(format!("{:?}", save_type))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(save_type, SaveType::Fortitude, "Fortitude");
                        ui.selectable_value(save_type, SaveType::Reflex, "Reflex");
                        ui.selectable_value(save_type, SaveType::Will, "Will");
                    });
            });
        }
        Mechanic::Guaranteed => {
            ui.label("（無額外參數）");
        }
    }
}

/// 渲染目標模式編輯表單
fn render_target_mode_form(ui: &mut egui::Ui, target_mode: &mut TargetMode, salt: &str) {
    let current_type = match target_mode {
        TargetMode::SingleTarget { .. } => TARGET_MODE_SINGLETARGET,
        TargetMode::MultiTarget { .. } => TARGET_MODE_MULTITARGET,
        TargetMode::Area { .. } => TARGET_MODE_AREA,
    };

    ui.horizontal(|ui| {
        ui.label("目標模式：");
        egui::ComboBox::from_id_salt(salt)
            .selected_text(current_type)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(
                        current_type == TARGET_MODE_SINGLETARGET,
                        TARGET_MODE_SINGLETARGET,
                    )
                    .clicked()
                {
                    *target_mode = TargetMode::SingleTarget {
                        filter: TargetFilter::All,
                    };
                }
                if ui
                    .selectable_label(
                        current_type == TARGET_MODE_MULTITARGET,
                        TARGET_MODE_MULTITARGET,
                    )
                    .clicked()
                {
                    *target_mode = TargetMode::MultiTarget {
                        count: 2,
                        allow_duplicate: true,
                        filter: TargetFilter::All,
                    };
                }
                if ui
                    .selectable_label(current_type == TARGET_MODE_AREA, TARGET_MODE_AREA)
                    .clicked()
                {
                    *target_mode = TargetMode::Area {
                        aoe_shape: AoeShape::Diamond { radius: 2 },
                        targets_unit: false,
                        filter: TargetFilter::All,
                    };
                }
            });
    });

    match target_mode {
        TargetMode::SingleTarget { filter } => {
            render_target_filter_form(ui, filter, salt);
        }
        TargetMode::MultiTarget {
            count,
            allow_duplicate,
            filter,
        } => {
            ui.horizontal(|ui| {
                ui.label("目標數量：");
                ui.add(
                    egui::DragValue::new(count)
                        .speed(DRAG_VALUE_SPEED)
                        .range(1..=i32::MAX),
                );
            });
            ui.horizontal(|ui| {
                ui.label("允許重複選擇：");
                ui.checkbox(allow_duplicate, "");
            });
            render_target_filter_form(ui, filter, salt);
        }
        TargetMode::Area {
            aoe_shape,
            targets_unit,
            filter,
        } => {
            ui.horizontal(|ui| {
                ui.label("以單位為目標：");
                ui.checkbox(targets_unit, "");
            });
            ui.separator();
            ui.label("AOE 形狀：");
            render_aoe_shape_form(ui, aoe_shape, salt);
            ui.separator();
            render_target_filter_form(ui, filter, salt);
        }
    }
}

/// 渲染單個效果的編輯表單
fn render_effect_form(ui: &mut egui::Ui, effect: &mut SkillEffect, effect_index: usize) {
    match effect {
        SkillEffect::HpModify {
            mechanic,
            target_mode,
            formula,
            style,
        } => {
            ui.label("類型：HP 修正");

            let hp_mechanic_salt = format!("effect_{}_hp_modify_mechanic", effect_index);
            render_mechanic_form(ui, mechanic, hp_mechanic_salt.as_str());
            ui.separator();
            let hp_target_salt = format!("effect_{}_hp_modify_target_mode", effect_index);
            render_target_mode_form(ui, target_mode, hp_target_salt.as_str());
            ui.separator();
            let hp_formula_salt = format!("effect_{}_hp_modify_formula", effect_index);
            render_formula_form(ui, formula, hp_formula_salt.as_str());
            ui.separator();

            let hp_style_salt = format!("effect_{}_hp_modify_style", effect_index);
            render_style_form(ui, style, &hp_style_salt);
        }
        SkillEffect::AttributeModify {
            mechanic,
            target_mode,
            formula,
            attribute,
            duration,
        } => {
            ui.label("類型：屬性修正");

            let attr_mechanic_salt = format!("effect_{}_attr_modify_mechanic", effect_index);
            render_mechanic_form(ui, mechanic, attr_mechanic_salt.as_str());
            ui.separator();
            let attr_target_salt = format!("effect_{}_attr_modify_target_mode", effect_index);
            render_target_mode_form(ui, target_mode, attr_target_salt.as_str());
            ui.separator();
            let attr_formula_salt = format!("effect_{}_attr_modify_formula", effect_index);
            render_formula_form(ui, formula, attr_formula_salt.as_str());
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("屬性：");
                egui::ComboBox::from_id_salt(format!(
                    "effect_{}_attr_modify_attribute",
                    effect_index
                ))
                .selected_text(format!("{:?}", attribute))
                .show_ui(ui, |ui| {
                    for attr_option in Attribute::iter() {
                        ui.selectable_value(attribute, attr_option, format!("{:?}", attr_option));
                    }
                });
            });
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("時效（回合，留空=永久）：");
                if let Some(d) = duration {
                    let mut d_val = *d;
                    if ui
                        .add(
                            egui::DragValue::new(&mut d_val)
                                .speed(DRAG_VALUE_SPEED)
                                .range(1..=i32::MAX),
                        )
                        .changed()
                    {
                        *d = d_val;
                    }
                    if ui.button("清除").clicked() {
                        *duration = None;
                    }
                } else {
                    if ui.button("設定時效").clicked() {
                        *duration = Some(1);
                    }
                }
            });
        }
        SkillEffect::Push {
            mechanic,
            target_mode,
            distance,
        } => {
            ui.label("類型：推離");

            let push_mechanic_salt = format!("effect_{}_push_mechanic", effect_index);
            render_mechanic_form(ui, mechanic, push_mechanic_salt.as_str());
            ui.separator();
            let push_target_salt = format!("effect_{}_push_target_mode", effect_index);
            render_target_mode_form(ui, target_mode, push_target_salt.as_str());
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("推離距離：");
                ui.add(
                    egui::DragValue::new(distance)
                        .speed(DRAG_VALUE_SPEED)
                        .range(1..=Coord::MAX),
                );
            });
        }
    }
}

/// 渲染數值計算公式編輯表單
fn render_formula_form(ui: &mut egui::Ui, formula: &mut ValueFormula, salt: &str) {
    let current_type = match formula {
        ValueFormula::Fixed { .. } => "Fixed",
        ValueFormula::Attribute { .. } => "Attribute",
    };

    ui.horizontal(|ui| {
        ui.label("計算方式：");
        egui::ComboBox::from_id_salt(salt)
            .selected_text(current_type)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(current_type == "Fixed", "Fixed")
                    .clicked()
                {
                    *formula = ValueFormula::Fixed { value: 0 };
                }
                if ui
                    .selectable_label(current_type == "Attribute", "Attribute")
                    .clicked()
                {
                    *formula = ValueFormula::Attribute {
                        source: AttributeSource::Caster,
                        attribute: Attribute::PhysicalAttack,
                        multiplier: 1.0,
                    };
                }
            });
    });

    match formula {
        ValueFormula::Fixed { value } => {
            ui.horizontal(|ui| {
                ui.label("數值：");
                ui.add(egui::DragValue::new(value).speed(DRAG_VALUE_SPEED));
            });
        }
        ValueFormula::Attribute {
            source,
            attribute,
            multiplier,
        } => {
            ui.horizontal(|ui| {
                ui.label("來源：");
                egui::ComboBox::from_id_salt(format!("{}_source", salt))
                    .selected_text(format!("{:?}", source))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(source, AttributeSource::Caster, "Caster");
                        ui.selectable_value(source, AttributeSource::Target, "Target");
                    });
            });

            ui.horizontal(|ui| {
                ui.label("屬性：");
                egui::ComboBox::from_id_salt(format!("{}_attr", salt))
                    .selected_text(format!("{:?}", attribute))
                    .show_ui(ui, |ui| {
                        for attr_option in Attribute::iter() {
                            ui.selectable_value(
                                attribute,
                                attr_option,
                                format!("{:?}", attr_option),
                            );
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("倍率：");
                ui.add(egui::DragValue::new(multiplier).speed(0.1));
            });
        }
    }
}

/// 渲染攻擊風格（傷害類型）選擇器
fn render_style_form(ui: &mut egui::Ui, style: &mut AttackStyle, salt: &str) {
    ui.horizontal(|ui| {
        ui.label("傷害類型：");
        egui::ComboBox::from_id_salt(salt)
            .selected_text(format!("{:?}", style))
            .show_ui(ui, |ui| {
                ui.selectable_value(style, AttackStyle::Physical, "Physical");
                ui.selectable_value(style, AttackStyle::Magical, "Magical");
            });
    });
}

/// 渲染目標過濾器編輯表單
fn render_target_filter_form(ui: &mut egui::Ui, filter: &mut TargetFilter, salt: &str) {
    ui.horizontal(|ui| {
        ui.label("目標過濾：");
        egui::ComboBox::from_id_salt(format!("{}_filter", salt))
            .selected_text(format!("{:?}", filter))
            .show_ui(ui, |ui| {
                ui.selectable_value(filter, TargetFilter::All, "所有單位");
                ui.selectable_value(filter, TargetFilter::Enemy, "敵人");
                ui.selectable_value(filter, TargetFilter::Ally, "友軍含施放者");
                ui.selectable_value(filter, TargetFilter::AllyExcludingCaster, "友軍不含施放者");
                ui.selectable_value(filter, TargetFilter::Caster, "施放者");
            });
    });
}

/// 渲染 AOE 形狀編輯表單
fn render_aoe_shape_form(ui: &mut egui::Ui, shape: &mut AoeShape, salt: &str) {
    let current_type = match shape {
        AoeShape::Diamond { .. } => AOE_SHAPE_DIAMOND,
        AoeShape::Cross { .. } => AOE_SHAPE_CROSS,
        AoeShape::Line { .. } => AOE_SHAPE_LINE,
        AoeShape::Rectangle { .. } => AOE_SHAPE_RECTANGLE,
    };

    ui.horizontal(|ui| {
        ui.label("形狀類型：");
        egui::ComboBox::from_id_salt(format!("{}_aoe_shape_type", salt))
            .selected_text(current_type)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(current_type == AOE_SHAPE_DIAMOND, AOE_SHAPE_DIAMOND)
                    .clicked()
                {
                    *shape = AoeShape::Diamond { radius: 2 };
                }
                if ui
                    .selectable_label(current_type == AOE_SHAPE_CROSS, AOE_SHAPE_CROSS)
                    .clicked()
                {
                    *shape = AoeShape::Cross { length: 2 };
                }
                if ui
                    .selectable_label(current_type == AOE_SHAPE_LINE, AOE_SHAPE_LINE)
                    .clicked()
                {
                    *shape = AoeShape::Line { length: 2 };
                }
                if ui
                    .selectable_label(current_type == AOE_SHAPE_RECTANGLE, AOE_SHAPE_RECTANGLE)
                    .clicked()
                {
                    *shape = AoeShape::Rectangle {
                        width: 2,
                        height: 1,
                    };
                }
            });
    });

    // 根據類型顯示對應的參數
    match shape {
        AoeShape::Diamond { radius } => {
            ui.horizontal(|ui| {
                ui.label("半徑：");
                ui.add(
                    egui::DragValue::new(radius)
                        .speed(DRAG_VALUE_SPEED)
                        .range(2..=Coord::MAX),
                );
            });
        }
        AoeShape::Cross { length } => {
            ui.horizontal(|ui| {
                ui.label("長度：");
                ui.add(
                    egui::DragValue::new(length)
                        .speed(DRAG_VALUE_SPEED)
                        .range(2..=Coord::MAX),
                );
            });
        }
        AoeShape::Line { length } => {
            ui.horizontal(|ui| {
                ui.label("長度：");
                ui.add(
                    egui::DragValue::new(length)
                        .speed(DRAG_VALUE_SPEED)
                        .range(2..=Coord::MAX),
                );
            });
        }
        AoeShape::Rectangle { width, height } => {
            ui.horizontal(|ui| {
                ui.label("寬度：");
                ui.add(
                    egui::DragValue::new(width)
                        .speed(DRAG_VALUE_SPEED)
                        .range(1..=Coord::MAX),
                );
            });
            ui.horizontal(|ui| {
                ui.label("高度：");
                ui.add(
                    egui::DragValue::new(height)
                        .speed(DRAG_VALUE_SPEED)
                        .range(1..=Coord::MAX),
                );
            });
        }
    }
}
