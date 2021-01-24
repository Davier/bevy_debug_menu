# Bevy Debug Menu
 
This crate adds a debug menu to your [Bevy](https://bevyengine.org) game. It can show your diagnostics, and explore and edit the current entities and resources.
 
## Usage

* make sure you are using `bevy` from a recent `master` branch
* add dependency in `Cargo.toml`:
```toml
[dependencies]
bevy_debug_menu = { git = "https://github.com/Davier/bevy_debug_menu" }
```
* add the `DebugMenuPlugin` to your bevy app
* launch the app and press `F10` to show or hide the menu


## Optional setup

* derive `Reflect`, `ReflectComponent` and `ReflectResource` on your types
* adapt your game to stop processing keyboard inputs when editing entities by listening to `FocusedEvent` and `UnfocusedEvent`
 
## Warnings

* you need to compile in release mode to have descent frame rate
* with default features, the debug menu will push the rest of your UI elements even when hidden (see [#1211](https://github.com/bevyengine/bevy/pull/1211))
* with default features, the "Resource" panel is disabled (see [#1260](https://github.com/bevyengine/bevy/pull/1260))
* both issues can be fixed by using a bevy fork and enabling the `extra` feature in `Cargo.toml`:
```toml
[dependencies]
bevy_debug_menu = { git = "https://github.com/Davier/bevy_debug_menu", features = ["extra"] }
[patch.crates-io]
bevy = { git = "https://github.com/Davier/bevy", branch = "extra" }
```
 
## Example

```rust
use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(bevy_debug_menu::DebugMenuPlugin)
        // This is unnecessary if you already initialize the UI camera in your game
        .add_startup_system(bevy_debug_menu::setup_ui_camera.system())
        .run();
}
```

## TODO

* search by resource type, component type, entity Name and entity Label
* remove resource, entity, component
* load/save scene
* list and spawn loaded scenes
* integration with bevy_mod_picking
* implement reflection for enums