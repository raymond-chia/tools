use godot::prelude::*;

mod battle_root;
mod board_error;

struct GodotBindExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotBindExtension {}
