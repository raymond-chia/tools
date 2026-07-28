use godot::classes::{Control, IControl};
use godot::prelude::*;

/// 戰鬥 UI 的根節點。階段 A 只驗證 dll 能被 Godot 載入，尚無任何狀態與子節點。
#[derive(GodotClass)]
#[class(init, base = Control)]
pub struct BattleRoot {
    base: Base<Control>,
}

#[godot_api]
impl IControl for BattleRoot {
    fn ready(&mut self) {
        godot_print!("BattleRoot ready");
    }
}
