//! 技能編輯器 tab

use crate::constants::*;
use crate::editor_item::{EditorItem, validate_name};
use crate::generic_editor::MessageState;
use crate::utils::dnd::render_dnd_handle;
use board::domain::core_types::{
    Area, Attribute, BuffType, ContinuousEffect, Effect, EffectCondition, EffectNode, EndCondition,
    Scaling, SkillTag, SkillType, Target, TriggeringSource,
};
use std::fmt::Display;
use strum::IntoEnumIterator;

// ==================== EditorItem 實作 ====================

impl EditorItem for SkillType {
    type UIState = ();

    fn name(&self) -> &str {
        &self.name()
    }

    fn set_name(&mut self, name: String) {
        match self {
            Self::Active { name: n, .. } => *n = name,
            Self::Reaction { name: n, .. } => *n = name,
            Self::Passive { name: n, .. } => *n = name,
        }
    }

    fn type_name() -> &'static str {
        "技能"
    }

    fn validate(&self, all_items: &[Self], editing_index: Option<usize>) -> Result<(), String> {
        validate_name(self, all_items, editing_index)?;
        Ok(())
    }
}

/// 取得技能的檔案名稱
pub fn file_name() -> &'static str {
    "skills"
}

// ==================== 通用輔助函數 ====================

/// 通用 enum 下拉選單，使用 Display 比較 discriminant
fn enum_combo_box<E>(ui: &mut egui::Ui, label: &str, current: &mut E, id_salt: &str)
where
    E: IntoEnumIterator + Default + Display + Clone,
{
    ui.horizontal(|ui| {
        ui.label(label);
        let current_label = current.to_string();
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(&current_label)
            .show_ui(ui, |ui| {
                for variant in E::iter() {
                    let variant_label = variant.to_string();
                    let selected = variant_label == current_label;
                    if ui.selectable_label(selected, &variant_label).clicked() {
                        *current = variant;
                    }
                }
            });
    });
}

/// 通用 enum variant 新增按鈕（Grid 排列）
fn enum_add_buttons<E>(ui: &mut egui::Ui, items: &mut Vec<E>, id_salt: &str, columns: usize)
where
    E: IntoEnumIterator + Display,
{
    egui::Grid::new(id_salt)
        .num_columns(columns)
        .show(ui, |ui| {
            for (idx, variant) in E::iter().enumerate() {
                if ui.button(format!("+ {}", variant)).clicked() {
                    items.push(variant);
                }
                if (idx + 1) % columns == 0 {
                    ui.end_row();
                }
            }
        });
}

/// 渲染數值輸入
fn drag_value<N: egui::emath::Numeric>(ui: &mut egui::Ui, label: &str, value: &mut N) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(value).speed(DRAG_VALUE_SPEED));
    });
}

/// 渲染數值範圍輸入
fn pair_drag_value<N: egui::emath::Numeric>(ui: &mut egui::Ui, label: &str, pair: &mut (N, N)) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(&mut pair.0).speed(DRAG_VALUE_SPEED));
        ui.label("~");
        ui.add(egui::DragValue::new(&mut pair.1).speed(DRAG_VALUE_SPEED));
    });
}

/// 渲染刪除按鈕，回傳是否點擊
fn delete_button(ui: &mut egui::Ui) -> bool {
    ui.button("x").clicked()
}

/// 渲染簡單 Vec 的新增/刪除列表
fn render_simple_vec<E, F>(
    ui: &mut egui::Ui,
    label: &str,
    items: &mut Vec<E>,
    id_salt: &str,
    render_item: F,
) where
    F: Fn(&mut egui::Ui, &mut E, &str),
{
    ui.label(label);
    let mut to_remove = None;
    for (idx, item) in items.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            if delete_button(ui) {
                to_remove = Some(idx);
            }
            render_item(ui, item, &format!("{id_salt}_{idx}"));
        });
    }
    if let Some(idx) = to_remove {
        items.remove(idx);
    }
}

// ==================== 表單渲染 ====================

/// 渲染技能編輯表單
pub fn render_form(
    ui: &mut egui::Ui,
    skill: &mut SkillType,
    _ui_state: &mut (),
    _message_state: &mut MessageState,
) {
    // 步驟 2：variant 切換
    render_variant_selector(ui, skill);

    ui.add_space(SPACING_SMALL);

    // 共用欄位：名稱
    let mut name = skill.name().clone();
    ui.horizontal(|ui| {
        ui.label("名稱：");
        ui.text_edit_singleline(&mut name);
    });
    skill.set_name(name);

    // 共用欄位：tags
    render_skill_tags(ui, skill);

    // cost（Passive 沒有）
    match skill {
        SkillType::Active { cost, .. } | SkillType::Reaction { cost, .. } => {
            drag_value(ui, "消耗：", cost);
        }
        SkillType::Passive { .. } => {}
    }

    ui.add_space(SPACING_SMALL);
    ui.separator();

    // variant 專屬欄位
    match skill {
        SkillType::Active {
            target, effects, ..
        } => {
            ui.heading("目標設定");
            render_target(ui, target);
            ui.add_space(SPACING_SMALL);
            ui.separator();
            ui.heading("效果節點");
            render_effect_node_list(ui, effects, "active_effects", 0);
        }
        SkillType::Reaction {
            triggering_unit,
            effects,
            ..
        } => {
            ui.heading("觸發來源");
            render_triggering_source(ui, triggering_unit);
            ui.add_space(SPACING_SMALL);
            ui.separator();
            ui.heading("效果節點");
            render_effect_node_list(ui, effects, "reaction_effects", 0);
        }
        SkillType::Passive { effects, .. } => {
            ui.heading("持續效果");
            render_continuous_effect_list(ui, effects, "passive_effects");
        }
    }
}

/// 渲染 variant 選擇器
fn render_variant_selector(ui: &mut egui::Ui, skill: &mut SkillType) {
    ui.horizontal(|ui| {
        ui.label("類型：");
        let current_label = skill.to_string();
        let current_name = skill.name().clone();

        for variant in SkillType::iter() {
            let variant_label = variant.to_string();
            let selected = variant_label == current_label;
            if ui.selectable_label(selected, &variant_label).clicked() && !selected {
                let mut new_skill = variant;
                new_skill.set_name(current_name.clone());
                *skill = new_skill;
            }
        }
    });
}

/// 渲染技能標籤列表
fn render_skill_tags(ui: &mut egui::Ui, skill: &mut SkillType) {
    let tags = match skill {
        SkillType::Active { tags, .. } => tags,
        SkillType::Reaction { tags, .. } => tags,
        SkillType::Passive { tags, .. } => tags,
    };

    render_simple_vec(ui, "標籤：", tags, "skill_tags", |ui, tag, id| {
        enum_combo_box(ui, "", tag, id);
    });

    if ui.button("+ 新增標籤").clicked() {
        tags.push(SkillTag::default());
    }
}

// ==================== 步驟 3：Target ====================

fn render_target(ui: &mut egui::Ui, target: &mut Target) {
    pair_drag_value(ui, "射程：", &mut target.range);

    enum_combo_box(ui, "選擇方式：", &mut target.selection, "target_selection");
    enum_combo_box(
        ui,
        "目標過濾：",
        &mut target.selectable_filter,
        "target_filter",
    );

    drag_value(ui, "目標數量：", &mut target.count);

    ui.horizontal(|ui| {
        ui.label("允許重複目標：");
        ui.checkbox(&mut target.allow_same_target, "");
    });

    render_area(ui, &mut target.area, "target_area");
}

/// 渲染 Area enum（有額外欄位的下拉）
fn render_area(ui: &mut egui::Ui, area: &mut Area, id_salt: &str) {
    enum_combo_box(ui, "範圍：", area, id_salt);

    match area {
        Area::Single => {}
        Area::Diamond { radius } => {
            drag_value(ui, "  半徑：", radius);
        }
        Area::Cross { length } => {
            drag_value(ui, "  長度：", length);
        }
        Area::Line { length } => {
            drag_value(ui, "  長度：", length);
        }
    }
}

// ==================== 步驟 4：TriggeringSource ====================

fn render_triggering_source(ui: &mut egui::Ui, source: &mut TriggeringSource) {
    pair_drag_value(ui, "觸發範圍：", &mut source.source_range);

    enum_combo_box(ui, "來源過濾：", &mut source.source_filter, "source_filter");
    enum_combo_box(ui, "觸發條件：", &mut source.trigger, "source_trigger");
}

// ==================== 步驟 5：ContinuousEffect ====================

/// 通用可排序列表：拖放、刪除、摺疊 header
fn render_reorderable_list<T: Display>(
    ui: &mut egui::Ui,
    items: &mut Vec<T>,
    id_salt: &str,
    render_item: impl Fn(&mut egui::Ui, &mut T, &str),
) {
    let mut to_remove = None;
    let mut dnd_result = None;

    for (idx, item) in items.iter_mut().enumerate() {
        let item_id = egui::Id::new(id_salt).with(idx);
        let header_label = format!("#{} {}", idx, item);

        ui.horizontal(|ui| {
            if let Some(result) = render_dnd_handle(ui, item_id, idx, "☰") {
                dnd_result = Some(result);
            }
            if delete_button(ui) {
                to_remove = Some(idx);
            }
        });

        let child_id = format!("{id_salt}_{idx}");
        egui::CollapsingHeader::new(header_label)
            .id_salt(&child_id)
            .default_open(true)
            .show(ui, |ui| {
                render_item(ui, item, &child_id);
            });
    }

    if let Some(idx) = to_remove {
        items.remove(idx);
    }
    if let Some((from, to)) = dnd_result {
        if from != to && from < items.len() && to < items.len() {
            let item = items.remove(from);
            items.insert(to, item);
        }
    }
}

fn render_continuous_effect_list(
    ui: &mut egui::Ui,
    effects: &mut Vec<ContinuousEffect>,
    id_salt: &str,
) {
    render_reorderable_list(ui, effects, id_salt, |ui, effect, child_id| {
        render_continuous_effect(ui, effect, child_id);
    });

    if ui.button("+ 種族屬性").clicked() {
        for attr in Attribute::iter() {
            effects.push(ContinuousEffect::AttributeFlat {
                attribute: attr,
                value: 0,
            });
        }
    }
    enum_add_buttons(ui, effects, &format!("{id_salt}_add"), 3);
}

fn render_continuous_effect(ui: &mut egui::Ui, effect: &mut ContinuousEffect, id_salt: &str) {
    match effect {
        ContinuousEffect::AttributeFlat { attribute, value } => {
            enum_combo_box(ui, "屬性：", attribute, &format!("{id_salt}_attr"));
            drag_value(ui, "數值：", value);
        }
        ContinuousEffect::AttributeScaling {
            target_attribute,
            source,
            source_attribute,
            value_percent,
        } => {
            enum_combo_box(
                ui,
                "目標屬性：",
                target_attribute,
                &format!("{id_salt}_tattr"),
            );
            enum_combo_box(ui, "來源：", source, &format!("{id_salt}_src"));
            enum_combo_box(
                ui,
                "來源屬性：",
                source_attribute,
                &format!("{id_salt}_sattr"),
            );
            drag_value(ui, "百分比：", value_percent);
        }
        ContinuousEffect::NearbyAllyScaling {
            range,
            attribute,
            per_ally_percent,
            base_percent,
        } => {
            drag_value(ui, "範圍：", range);
            enum_combo_box(ui, "屬性：", attribute, &format!("{id_salt}_attr"));
            drag_value(ui, "每位盟友百分比：", per_ally_percent);
            drag_value(ui, "基礎百分比：", base_percent);
        }
        ContinuousEffect::HpRatioScaling {
            attribute,
            min_bonus_percent,
            step_percent,
            bonus_per_step,
            max_bonus_percent,
        } => {
            enum_combo_box(ui, "屬性：", attribute, &format!("{id_salt}_attr"));
            drag_value(ui, "最小加成%：", min_bonus_percent);
            drag_value(ui, "每級%：", step_percent);
            drag_value(ui, "每級加成：", bonus_per_step);
            drag_value(ui, "最大加成%：", max_bonus_percent);
        }
        ContinuousEffect::Perception {
            perception_type,
            range,
        } => {
            enum_combo_box(
                ui,
                "感知類型：",
                perception_type,
                &format!("{id_salt}_ptype"),
            );
            drag_value(ui, "範圍：", range);
        }
        ContinuousEffect::DamageToMp { ratio_percent } => {
            drag_value(ui, "轉換比例%：", ratio_percent);
        }
        ContinuousEffect::EmitLight { light_type, range } => {
            enum_combo_box(ui, "光源類型：", light_type, &format!("{id_salt}_ltype"));
            drag_value(ui, "範圍：", range);
        }
        ContinuousEffect::Blinded => {
            ui.label("（無額外欄位）");
        }
    }
}

// ==================== 步驟 6：EffectNode ====================

fn render_effect_node_list(
    ui: &mut egui::Ui,
    nodes: &mut Vec<EffectNode>,
    id_salt: &str,
    depth: usize,
) {
    render_reorderable_list(ui, nodes, id_salt, |ui, node, child_id| {
        render_effect_node(ui, node, child_id, depth);
    });

    enum_add_buttons(ui, nodes, &format!("{id_salt}_add"), 3);
}

fn render_effect_node(ui: &mut egui::Ui, node: &mut EffectNode, id_salt: &str, depth: usize) {
    match node {
        EffectNode::Area {
            area,
            filter,
            nodes,
        } => {
            render_area(ui, area, &format!("{id_salt}_area"));
            enum_combo_box(ui, "過濾：", filter, &format!("{id_salt}_filter"));
            ui.add_space(SPACING_SMALL);
            ui.label("子節點：");
            ui.indent(format!("{id_salt}_children"), |ui| {
                render_effect_node_list(ui, nodes, &format!("{id_salt}_nodes"), depth + 1);
            });
        }
        EffectNode::Branch {
            who,
            condition,
            on_success,
            on_failure,
        } => {
            enum_combo_box(ui, "判定對象：", who, &format!("{id_salt}_who"));
            render_effect_condition(ui, condition, &format!("{id_salt}_cond"));
            ui.add_space(SPACING_SMALL);
            ui.label("成功時：");
            ui.indent(format!("{id_salt}_success"), |ui| {
                render_effect_node_list(ui, on_success, &format!("{id_salt}_succ"), depth + 1);
            });
            ui.label("失敗時：");
            ui.indent(format!("{id_salt}_failure"), |ui| {
                render_effect_node_list(ui, on_failure, &format!("{id_salt}_fail"), depth + 1);
            });
        }
        EffectNode::Leaf { who, effect } => {
            enum_combo_box(ui, "效果對象：", who, &format!("{id_salt}_who"));
            render_effect(ui, effect, &format!("{id_salt}_effect"));
        }
    }
}

// ==================== 步驟 7：Effect ====================

fn render_effect(ui: &mut egui::Ui, effect: &mut Effect, id_salt: &str) {
    enum_combo_box(ui, "效果類型：", effect, &format!("{id_salt}_type"));

    ui.add_space(SPACING_SMALL);

    match effect {
        Effect::HpEffect { scaling } => {
            render_scaling(ui, scaling, &format!("{id_salt}_scaling"));
        }
        Effect::MpEffect { value } => {
            drag_value(ui, "MP 值：", value);
        }
        Effect::ApplyBuff { buff } => {
            render_buff_type(ui, buff, &format!("{id_salt}_buff"));
        }
        Effect::ForcedMove {
            direction,
            distance,
        } => {
            enum_combo_box(ui, "方向：", direction, &format!("{id_salt}_dir"));
            drag_value(ui, "距離：", distance);
        }
        Effect::Trample { distance, scaling } => {
            drag_value(ui, "距離：", distance);
            render_scaling(ui, scaling, &format!("{id_salt}_scaling"));
        }
        Effect::SpawnObject {
            object_type,
            duration,
            contact_effects,
        } => {
            ui.horizontal(|ui| {
                ui.label("物件類型：");
                ui.text_edit_singleline(object_type);
            });
            ui.horizontal(|ui| {
                let mut has_duration = duration.is_some();
                ui.checkbox(&mut has_duration, "持續時間");
                match (has_duration, duration.as_mut()) {
                    (true, Some(dur)) => {
                        drag_value(ui, "", dur);
                    }
                    (true, None) => {
                        *duration = Some(1);
                    }
                    (false, _) => {
                        *duration = None;
                    }
                }
            });
            ui.label("接觸效果：");
            ui.indent(format!("{id_salt}_contact"), |ui| {
                render_effect_node_list(
                    ui,
                    contact_effects,
                    &format!("{id_salt}_contact_nodes"),
                    0,
                );
            });
        }
        Effect::AllowRemainingMovement | Effect::SwapPosition => {
            ui.label("（無額外欄位）");
        }
    }
}

/// 渲染 Scaling 結構
fn render_scaling(ui: &mut egui::Ui, scaling: &mut Scaling, id_salt: &str) {
    enum_combo_box(ui, "來源：", &mut scaling.source, &format!("{id_salt}_src"));
    enum_combo_box(
        ui,
        "來源屬性：",
        &mut scaling.source_attribute,
        &format!("{id_salt}_attr"),
    );
    drag_value(ui, "百分比：", &mut scaling.value_percent);
}

// ==================== 步驟 8：BuffType ====================

fn render_buff_type(ui: &mut egui::Ui, buff: &mut BuffType, id_salt: &str) {
    ui.horizontal(|ui| {
        ui.label("可疊加：");
        ui.checkbox(&mut buff.stackable, "");
    });

    ui.add_space(SPACING_SMALL);
    ui.label("持續效果：");
    ui.indent(format!("{id_salt}_while"), |ui| {
        render_continuous_effect_list(ui, &mut buff.while_active, &format!("{id_salt}_while"));
    });

    ui.add_space(SPACING_SMALL);
    ui.label("每回合效果：");
    ui.indent(format!("{id_salt}_perturn"), |ui| {
        render_effect_node_list(
            ui,
            &mut buff.per_turn_effects,
            &format!("{id_salt}_perturn"),
            0,
        );
    });

    ui.add_space(SPACING_SMALL);
    render_end_condition_list(ui, &mut buff.end_conditions, &format!("{id_salt}_endcond"));
}

// ==================== 步驟 9：EndCondition ====================

fn render_end_condition_list(ui: &mut egui::Ui, conditions: &mut Vec<EndCondition>, id_salt: &str) {
    render_simple_vec(ui, "結束條件：", conditions, id_salt, render_end_condition);
    enum_add_buttons(ui, conditions, &format!("{id_salt}_add"), 3);
}

fn render_end_condition(ui: &mut egui::Ui, condition: &mut EndCondition, id_salt: &str) {
    match condition {
        EndCondition::Duration(duration) => {
            drag_value(ui, "Duration：", duration);
        }
        EndCondition::CasterUsesSkillWithoutTag(tag) => {
            ui.label("CasterUsesSkillWithoutTag：");
            enum_combo_box(ui, "", tag, &format!("{id_salt}_tag"));
        }
        EndCondition::TargetSavesPerTurn
        | EndCondition::EitherDies
        | EndCondition::EitherMoves
        | EndCondition::TargetMoves => {
            ui.label(condition.to_string());
        }
    }
}

// ==================== 步驟 10：EffectCondition ====================

fn render_effect_condition(ui: &mut egui::Ui, condition: &mut EffectCondition, id_salt: &str) {
    enum_combo_box(ui, "判定類型：", condition, &format!("{id_salt}_type"));

    match condition {
        EffectCondition::HitCheck {
            accuracy_bonus,
            crit_bonus,
        } => {
            drag_value(ui, "命中加成：", accuracy_bonus);
            drag_value(ui, "暴擊加成：", crit_bonus);
        }
        EffectCondition::DcCheck { dc_type, dc_bonus } => {
            enum_combo_box(ui, "DC 類型：", dc_type, &format!("{id_salt}_dctype"));
            drag_value(ui, "DC 加成：", dc_bonus);
        }
    }
}
