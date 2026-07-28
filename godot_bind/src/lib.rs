use godot::prelude::*;

mod battle_root;

struct GodotBindExtension;

#[gdextension]
unsafe impl ExtensionLibrary for GodotBindExtension {}
