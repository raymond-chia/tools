mod ecs_logic;

use godot::prelude::*;

struct GodotBind;

#[gdextension]
unsafe impl ExtensionLibrary for GodotBind {}
