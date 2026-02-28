# gdext 範例

```rust
use godot::classes::{ISprite2D, ProgressBar, Sprite2D};
use godot::prelude::*;

// Declare the Player class inheriting Sprite2D.
#[derive(GodotClass)]
#[class(init, base=Sprite2D)] // Automatic initialization, no manual init() needed.
struct Player {
    // Inheritance via composition: access to Sprite2D methods.
    base: Base<Sprite2D>,

    // #[class(init)] above allows attribute-initialization of fields.
    #[init(val = 100)]
    hitpoints: i32,

    // Access to a child node, auto-initialized when _ready() is called.
    #[init(node = "Ui/HealthBar")] // <- Path to the node in the scene tree.
    health_bar: OnReady<Gd<ProgressBar>>,
}

// Implement Godot's virtual methods via predefined trait.
#[godot_api]
impl ISprite2D for Player {
    // Override the `_ready` method.
    fn ready(&mut self) {
        godot_print!("Player ready!");

        // Health bar is already initialized and straightforward to access.
        self.health_bar.set_max(self.hitpoints as f64);
        self.health_bar.set_value(self.hitpoints as f64);

        // Connect type-safe signal: print whenever the health bar is updated.
        self.health_bar.signals().value_changed().connect(|hp| {
            godot_print!("Health changed to: {hp}");
        });
    }
}

// Implement custom methods that can be called from GDScript.
#[godot_api]
impl Player {
    #[func]
    fn take_damage(&mut self, damage: i32) {
        self.hitpoints -= damage;
        godot_print!("Player hit! HP left: {}", self.hitpoints);

        // Update health bar.
        self.health_bar.set_value(self.hitpoints as f64);

        // Call Node methods on self, via mutable base access.
        if self.hitpoints <= 0 {
            self.base_mut().queue_free();
        }
    }
}
```

# ai 提供範例 (還沒驗證)

```rust
#[derive(GodotClass)]
#[class(init, base=RefCounted)]
struct UnitInfo {
    base: Base<RefCounted>,
    #[init(val = 0)]
    hp: i32,
    #[init(val = 0)]
    mp: i32,
}

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
struct ErrorInfo {
    base: Base<RefCounted>,
    #[init(val = GString::new())]
    message: GString,
}

#[func]
fn get_unit(&self, unit_id: u32) -> Variant {
    match self.query_unit(unit_id) {
        Ok(data) => {
            let mut info = Gd::<UnitInfo>::default();
            info.bind_mut().hp = data.hp;
            info.bind_mut().mp = data.mp;
            info.to_variant()
        }
        Err(err) => {
            let mut info = Gd::<ErrorInfo>::default();
            info.bind_mut().message = err.to_string().into();
            info.to_variant()
        }
    }
}
```
