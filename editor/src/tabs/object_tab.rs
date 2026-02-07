//! 物件編輯器 tab

use crate::constants::DRAG_VALUE_SPEED;
use crate::editor_item::EditorItem;
use board::constants::{CONTACT_HEALTH_DAMAGE, IMPASSABLE_MOVEMENT_COST};
use board::loader_schema::ObjectType;

// ==================== EditorItem 實作 ====================

impl EditorItem for ObjectType {
    type UIState = ();

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn type_name() -> &'static str {
        "物件"
    }
}

/// 取得物件的檔案名稱
pub fn file_name() -> &'static str {
    "objects"
}

// ==================== 表單渲染 ====================

/// 渲染物件編輯表單
pub fn render_form(ui: &mut egui::Ui, obj: &mut ObjectType, _ui_state: &mut ()) {
    ui.horizontal(|ui| {
        ui.label("名稱：");
        ui.text_edit_singleline(&mut obj.name);
    });

    ui.horizontal(|ui| {
        ui.label("移動成本：");
        let mut cost = obj.movement_cost as i32;
        ui.add(
            egui::DragValue::new(&mut cost)
                .speed(DRAG_VALUE_SPEED)
                .range(0..=IMPASSABLE_MOVEMENT_COST as i32),
        );
        obj.movement_cost = cost as usize;
    });

    ui.horizontal(|ui| {
        ui.label("阻擋視線：");
        ui.checkbox(&mut obj.blocks_sight, "");
    });

    ui.horizontal(|ui| {
        ui.label("阻擋聲音：");
        ui.checkbox(&mut obj.blocks_sound, "");
    });

    ui.horizontal(|ui| {
        ui.label("立即 HP 效果：");
        ui.add(
            egui::DragValue::new(&mut obj.contact_health)
                .speed(DRAG_VALUE_SPEED)
                .range(-CONTACT_HEALTH_DAMAGE..=CONTACT_HEALTH_DAMAGE),
        );
    });
}
