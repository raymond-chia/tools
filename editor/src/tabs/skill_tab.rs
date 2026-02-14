//! 技能編輯器 tab

use crate::constants::*;
use crate::editor_item::{EditorItem, validate_name};
use board::alias::Coord;
use board::loader_schema::{
    AoeShape, AttackStyle, Attribute, AttributeSource, Mechanic, SaveType, SkillEffect, SkillType,
    TargetFilter, TargetMode, TriggerEvent, ValueFormula,
};
use std::fmt::Debug;
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

    fn validate(&self, all_items: &[Self], editing_index: Option<usize>) -> Result<(), String> {
        validate_name(self, all_items, editing_index)?;

        // 檢查 MP 消耗
        if self.mp_change > 0 {
            return Err("MP 消耗不能為正數".to_string());
        }

        // 檢查施放距離：min_range 必須小於等於 max_range
        if self.min_range > self.max_range {
            return Err(format!(
                "最小施放距離({}) 必須小於等於最大施放距離({})",
                self.min_range, self.max_range
            ));
        }

        match self.trigger {
            // 持續生效的被動 + 每回合觸發的被動
            TriggerEvent::Passive | TriggerEvent::TurnEnd => {
                if self.mp_change != 0 {
                    return Err("被動和回合結束技能不能消耗 MP".to_string());
                }
                if self.min_range != 0 || self.max_range != 0 {
                    return Err("被動和回合結束技能不能有施放距離".to_string());
                }
                if self.allows_movement_after {
                    return Err("被動和回合結束技能不能設定使用後可移動".to_string());
                }
            }
            TriggerEvent::Active
            | TriggerEvent::OnBeingAttacked { .. }
            | TriggerEvent::OnAdjacentUnitMove { .. } => {}
        }

        validate_skill_effects(&self.effects)
    }
}

/// 取得技能的檔案名稱
pub fn file_name() -> &'static str {
    "skills"
}

fn validate_skill_effects(effects: &[SkillEffect]) -> Result<(), String> {
    for (effect_index, effect) in effects.iter().enumerate() {
        match effect {
            SkillEffect::HpModify {
                formula: ValueFormula::Fixed { value },
                ..
            }
            | SkillEffect::AttributeModify {
                formula: ValueFormula::Fixed { value },
                ..
            } => {
                if *value == 0 {
                    return Err(format!(
                        "效果 #{} 的 HP 修正數值不能為 0，請刪除該效果或設定非零數值",
                        effect_index + 1
                    ));
                }
            }
            _ => {}
        }
        if let SkillEffect::AttributeModify {
            formula:
                ValueFormula::Attribute {
                    attribute: base_attr,
                    ..
                },
            attribute: target_attr,
            ..
        } = effect
        {
            if base_attr != target_attr {
                return Err(format!(
                    "效果 #{} 的屬性修正倍率：目標屬性 ({:?}) 必須與公式屬性 ({:?}) 相同",
                    effect_index + 1,
                    target_attr,
                    base_attr
                ));
            }
        }
        if let SkillEffect::AttributeModify {
            duration: Some(duration),
            ..
        } = effect
        {
            if *duration <= 0 {
                return Err(format!(
                    "效果 #{} 的屬性修正時效必須大於 0，請刪除該效果或設定有效的時效",
                    effect_index + 1
                ));
            }
        }
        if let SkillEffect::Push { distance, .. } = effect {
            if *distance == 0 {
                return Err(format!(
                    "效果 #{} 的推離距離不能為 0，請刪除該效果或設定非零距離",
                    effect_index + 1
                ));
            }
        }
    }
    Ok(())
}

// ==================== 表單渲染 ====================

/// 渲染技能編輯表單
pub fn render_form(ui: &mut egui::Ui, skill: &mut SkillType, _ui_state: &mut ()) {
    render_race_template_buttons(ui, skill);
    ui.add_space(SPACING_MEDIUM);

    render_basic_info(ui, skill);
    ui.add_space(SPACING_SMALL);
    render_tags_section(ui, skill);
    ui.add_space(SPACING_MEDIUM);

    ui.heading("技能效果");

    // 簡單被動技能使用簡化 UI，其他技能使用完整表單
    if is_simple_passive_skill(skill) {
        render_passive_attributes_form(ui, skill);
    } else {
        render_effect_add_buttons(ui, skill);
        ui.add_space(SPACING_SMALL);
        render_effect_list(ui, skill);
    }
}

/// 渲染種族模板快速創建按鈕
fn render_race_template_buttons(ui: &mut egui::Ui, skill: &mut SkillType) {
    ui.label("種族模板：");
    ui.horizontal(|ui| {
        if ui.button("種族被動").clicked() {
            *skill = create_race_skill();
        }
    });
}

/// 渲染被動技能屬性編輯面板（簡化版）
fn render_passive_attributes_form(ui: &mut egui::Ui, skill: &mut SkillType) {
    // 從 effects 提取數值（按固定順序）
    let mut values: Vec<i32> = skill
        .effects
        .iter()
        .map(|effect| {
            if let SkillEffect::AttributeModify {
                formula: ValueFormula::Fixed { value },
                ..
            } = effect
            {
                *value
            } else {
                0
            }
        })
        .collect();

    // 渲染屬性編輯表格
    egui::Grid::new("passive_attributes_grid")
        .num_columns(2)
        .spacing([10.0, 5.0])
        .striped(true)
        .show(ui, |ui| {
            for (attribute, value) in Attribute::iter().zip(values.iter_mut()) {
                ui.label(format!("{:?}", attribute));
                ui.add(
                    egui::DragValue::new(value)
                        .speed(DRAG_VALUE_SPEED)
                        .range(i32::MIN..=i32::MAX),
                );
                ui.end_row();
            }
        });

    ui.add_space(SPACING_SMALL);

    // 同步回 skill.effects（保持原順序）
    skill.effects = Attribute::iter()
        .zip(values.into_iter())
        .map(|(attribute, value)| create_caster_attribute_modify(attribute, value))
        .collect();
}

/// 渲染基本資訊區塊（名稱、MP、施放距離、移動後使用）
fn render_basic_info(ui: &mut egui::Ui, skill: &mut SkillType) {
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
        ui.label("施放距離：");
        ui.add(
            egui::DragValue::new(&mut skill.min_range)
                .speed(DRAG_VALUE_SPEED)
                .range(0..=Coord::MAX),
        );
        ui.add(
            egui::DragValue::new(&mut skill.max_range)
                .speed(DRAG_VALUE_SPEED)
                .range(0..=Coord::MAX),
        );
    });

    render_trigger_section(ui, skill);

    ui.horizontal(|ui| {
        ui.label("使用後可移動：");
        ui.checkbox(&mut skill.allows_movement_after, "");
    });
}

/// 渲染觸發條件區塊
fn render_trigger_section(ui: &mut egui::Ui, skill: &mut SkillType) {
    let salt = "skill_trigger";

    ui.horizontal(|ui| {
        ui.label("觸發類型：");
        egui::ComboBox::from_id_salt(salt)
            .selected_text(ui_string(&skill.trigger))
            .show_ui(ui, |ui| {
                for trigger_option in TriggerEvent::iter() {
                    let option = match trigger_option {
                        TriggerEvent::Active => TriggerEvent::Active,
                        TriggerEvent::Passive => TriggerEvent::Passive,
                        TriggerEvent::TurnEnd => TriggerEvent::TurnEnd,
                        TriggerEvent::OnBeingAttacked { .. } => TriggerEvent::OnBeingAttacked {
                            attacker_filter: Default::default(),
                        },
                        TriggerEvent::OnAdjacentUnitMove { .. } => {
                            TriggerEvent::OnAdjacentUnitMove {
                                unit_filter: Default::default(),
                            }
                        }
                    };
                    ui.selectable_value(&mut skill.trigger, option.clone(), ui_string(&option));
                }
            });
    });

    // 根據類型顯示對應的參數
    match &mut skill.trigger {
        TriggerEvent::Active | TriggerEvent::Passive | TriggerEvent::TurnEnd => {}
        TriggerEvent::OnBeingAttacked { attacker_filter } => {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("攻擊者過濾：");
                egui::ComboBox::from_id_salt(format!("{}_attacker_filter", salt))
                    .selected_text(ui_string(&attacker_filter))
                    .show_ui(ui, |ui| {
                        for option in TargetFilter::iter() {
                            ui.selectable_value(
                                attacker_filter,
                                option.clone(),
                                ui_string(&option),
                            );
                        }
                    });
            });
        }
        TriggerEvent::OnAdjacentUnitMove { unit_filter } => {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("單位過濾：");
                egui::ComboBox::from_id_salt(format!("{}_unit_filter", salt))
                    .selected_text(ui_string(&unit_filter))
                    .show_ui(ui, |ui| {
                        for option in TargetFilter::iter() {
                            ui.selectable_value(unit_filter, option.clone(), ui_string(&option));
                        }
                    });
            });
        }
    }
}

/// 渲染標籤區塊
fn render_tags_section(ui: &mut egui::Ui, skill: &mut SkillType) {
    ui.label("標籤（以逗號分隔）：");
    let mut tags_text = skill.tags.join(", ");
    ui.text_edit_singleline(&mut tags_text);
    skill.tags = tags_text
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
}

/// 渲染新增效果按鈕
fn render_effect_add_buttons(ui: &mut egui::Ui, skill: &mut SkillType) {
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
}

/// 渲染效果列表
fn render_effect_list(ui: &mut egui::Ui, skill: &mut SkillType) {
    let mut to_remove = None;
    for (index, effect) in skill.effects.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                if ui.button("刪除").clicked() {
                    to_remove = Some(index);
                }
                ui.label(format!("效果 #{}", index + 1));
            });
            ui.separator();
            render_effect_form(ui, effect, index);
        });
        ui.add_space(SPACING_SMALL);
    }

    // 刪除標記的效果
    if let Some(index) = to_remove {
        skill.effects.remove(index);
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
                .selected_text(ui_string(attribute))
                .show_ui(ui, |ui| {
                    for attr_option in Attribute::iter() {
                        ui.selectable_value(attribute, attr_option, ui_string(&attr_option));
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

/// 渲染判定機制編輯表單
fn render_mechanic_form(ui: &mut egui::Ui, mechanic: &mut Mechanic, salt: &str) {
    ui.horizontal(|ui| {
        ui.label("判定機制：");
        egui::ComboBox::from_id_salt(salt)
            .selected_text(ui_string(mechanic))
            .show_ui(ui, |ui| {
                for mechanic_option in Mechanic::iter() {
                    let option = match mechanic_option {
                        Mechanic::HitBased { .. } => Mechanic::HitBased {
                            hit_bonus: 80,
                            crit_rate: 5,
                        },
                        Mechanic::DcBased { .. } => Mechanic::DcBased {
                            dc: 0,
                            save_type: SaveType::Fortitude,
                        },
                        Mechanic::Guaranteed => Mechanic::Guaranteed,
                    };
                    ui.selectable_value(mechanic, option.clone(), ui_string(&option));
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
                    .selected_text(ui_string(save_type))
                    .show_ui(ui, |ui| {
                        for option in SaveType::iter() {
                            ui.selectable_value(save_type, option.clone(), ui_string(&option));
                        }
                    });
            });
        }
        Mechanic::Guaranteed => {}
    }
}

/// 渲染目標模式編輯表單
fn render_target_mode_form(ui: &mut egui::Ui, target_mode: &mut TargetMode, salt: &str) {
    ui.horizontal(|ui| {
        ui.label("目標模式：");
        egui::ComboBox::from_id_salt(salt)
            .selected_text(ui_string(target_mode))
            .show_ui(ui, |ui| {
                for target_mode_option in TargetMode::iter() {
                    let option = match target_mode_option {
                        TargetMode::SingleTarget { .. } => TargetMode::SingleTarget {
                            filter: Default::default(),
                        },
                        TargetMode::MultiTarget { .. } => TargetMode::MultiTarget {
                            count: 2,
                            allow_duplicate: true,
                            filter: Default::default(),
                        },
                        TargetMode::Area { .. } => TargetMode::Area {
                            aoe_shape: Default::default(),
                            targets_unit: false,
                            filter: Default::default(),
                        },
                    };
                    ui.selectable_value(target_mode, option.clone(), ui_string(&option));
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

/// 渲染數值計算公式編輯表單
fn render_formula_form(ui: &mut egui::Ui, formula: &mut ValueFormula, salt: &str) {
    ui.horizontal(|ui| {
        ui.label("計算方式：");
        egui::ComboBox::from_id_salt(salt)
            .selected_text(ui_string(formula))
            .show_ui(ui, |ui| {
                for formula_option in ValueFormula::iter() {
                    let option = match formula_option {
                        ValueFormula::Fixed { .. } => ValueFormula::Fixed { value: 0 },
                        ValueFormula::Attribute { .. } => ValueFormula::Attribute {
                            source: AttributeSource::Caster,
                            attribute: Attribute::PhysicalAttack,
                            multiplier: 100,
                        },
                    };
                    ui.selectable_value(formula, option.clone(), ui_string(&option));
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
                    .selected_text(ui_string(source))
                    .show_ui(ui, |ui| {
                        for option in AttributeSource::iter() {
                            ui.selectable_value(source, option.clone(), ui_string(&option));
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("屬性：");
                egui::ComboBox::from_id_salt(format!("{}_attr", salt))
                    .selected_text(ui_string(attribute))
                    .show_ui(ui, |ui| {
                        for attr_option in Attribute::iter() {
                            ui.selectable_value(attribute, attr_option, ui_string(&attr_option));
                        }
                    });
            });

            ui.horizontal(|ui| {
                ui.label("倍率：");
                ui.add(
                    egui::DragValue::new(multiplier)
                        .speed(DRAG_VALUE_SPEED)
                        .range(0..=i32::MAX),
                );
            });
        }
    }
}

/// 渲染攻擊風格（傷害類型）選擇器
fn render_style_form(ui: &mut egui::Ui, style: &mut AttackStyle, salt: &str) {
    ui.horizontal(|ui| {
        ui.label("傷害類型：");
        egui::ComboBox::from_id_salt(salt)
            .selected_text(ui_string(style))
            .show_ui(ui, |ui| {
                for option in AttackStyle::iter() {
                    ui.selectable_value(style, option.clone(), ui_string(&option));
                }
            });
    });
}

/// 渲染目標過濾器編輯表單
fn render_target_filter_form(ui: &mut egui::Ui, filter: &mut TargetFilter, salt: &str) {
    ui.horizontal(|ui| {
        ui.label("目標過濾：");
        egui::ComboBox::from_id_salt(format!("{}_filter", salt))
            .selected_text(ui_string(filter))
            .show_ui(ui, |ui| {
                for option in TargetFilter::iter() {
                    ui.selectable_value(filter, option.clone(), ui_string(&option));
                }
            });
    });
}

/// 渲染 AOE 形狀編輯表單
fn render_aoe_shape_form(ui: &mut egui::Ui, shape: &mut AoeShape, salt: &str) {
    ui.horizontal(|ui| {
        ui.label("形狀類型：");
        egui::ComboBox::from_id_salt(format!("{}_aoe_shape_type", salt))
            .selected_text(ui_string(shape))
            .show_ui(ui, |ui| {
                for aoe_shape_option in AoeShape::iter() {
                    let option = match aoe_shape_option {
                        AoeShape::Diamond { .. } => AoeShape::Diamond { radius: 2 },
                        AoeShape::Cross { .. } => AoeShape::Cross { length: 2 },
                        AoeShape::Line { .. } => AoeShape::Line { length: 2 },
                        AoeShape::Rectangle { .. } => AoeShape::Rectangle {
                            width: 2,
                            height: 1,
                        },
                    };
                    ui.selectable_value(shape, option.clone(), ui_string(&option));
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

fn ui_string<T: Debug>(value: &T) -> String {
    format!("{:?}", value)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string()
}

/// 創建種族被動技能模板
pub fn create_race_skill() -> SkillType {
    SkillType {
        name: "".to_string(),
        mp_change: 0,
        min_range: 0,
        max_range: 0,
        trigger: TriggerEvent::Passive,
        tags: vec![],
        allows_movement_after: false,
        effects: [
            (Attribute::Hp, 80),
            (Attribute::Mp, 10),
            (Attribute::Initiative, 0),
            (Attribute::Hit, 0),
            (Attribute::Evasion, 0),
            (Attribute::Block, 0),
            (Attribute::BlockProtection, 0),
            (Attribute::PhysicalAttack, 0),
            (Attribute::MagicalAttack, 0),
            (Attribute::MagicalDc, 0),
            (Attribute::Fortitude, 0),
            (Attribute::Reflex, 0),
            (Attribute::Will, 0),
            (Attribute::Movement, 50),
            (Attribute::OpportunityAttacks, 0),
        ]
        .into_iter()
        .map(|(attribute, value)| create_caster_attribute_modify(attribute, value))
        .collect(),
    }
}

/// 判斷技能是否為簡單被動技能（被動且 effects 按屬性順序排列，且使用標準配置）
fn is_simple_passive_skill(skill: &SkillType) -> bool {
    if !matches!(skill.trigger, TriggerEvent::Passive) {
        return false;
    }

    // 檢查 effects 數量
    let expected_attrs: Vec<Attribute> = Attribute::iter().collect();
    if skill.effects.len() != expected_attrs.len() {
        return false;
    }

    // 檢查每個 effect 是否符合標準配置
    skill
        .effects
        .iter()
        .zip(expected_attrs.iter())
        .all(|(effect, expected_attr)| {
            matches!(
                effect,
                // 跟 create_caster_attribute_modify 同步
                SkillEffect::AttributeModify {
                    mechanic: Mechanic::Guaranteed,
                    target_mode: TargetMode::SingleTarget {
                        filter: TargetFilter::Caster
                    },
                    formula: ValueFormula::Fixed { .. },
                    attribute,
                    duration: None,
                    // guard clause
                } if *attribute == *expected_attr
            )
        })
}

// 跟 is_simple_passive_skill 同步
fn create_caster_attribute_modify(attribute: Attribute, value: i32) -> SkillEffect {
    SkillEffect::AttributeModify {
        mechanic: Mechanic::Guaranteed,
        target_mode: TargetMode::SingleTarget {
            filter: TargetFilter::Caster,
        },
        formula: ValueFormula::Fixed { value },
        attribute,
        duration: None,
    }
}
