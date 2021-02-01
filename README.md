# Bevy Debug Menu
 
[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-master-lightblue)](https://github.com/bevyengine/bevy/blob/master/docs/plugins_guidelines.md#master-branch-tracking)

This crate adds a debug menu to your [Bevy](https://bevyengine.org) game. It can show your diagnostics, and explore and edit the current entities and resources.
 
## Usage

* use `bevy` from a recent `master` branch
```toml
[patch.crates-io]
bevy = { git = "https://github.com/bevyengine/bevy", rev = "e6e23fdfa97b0ebfad3495407d9dff27d75ab843" }
```
* add `bevy_debug_menu` as a dependency
```toml
[dependencies]
bevy_debug_menu = { git = "https://github.com/Davier/bevy_debug_menu" }
```
* add the `DebugMenuPlugin` to your bevy app
* launch the app and press `F10` to show or hide the menu


## Optional setup

* derive `Reflect`, `ReflectComponent` and `ReflectResource` on your types
* adapt your game to stop processing keyboard inputs when editing entities by listening to `FocusedEvent` and `UnfocusedEvent`
* you need to use release mode to have descent frame rate, at least for your dependencies:
```toml
[profile.dev.package."*"]
opt-level = 3
```
* reflection on resources and enum types can be enabled with a bevy fork (see [#1260](https://github.com/bevyengine/bevy/pull/1260) and [#1347](https://github.com/bevyengine/bevy/pull/1347)):
```toml
[dependencies]
bevy_debug_menu = { git = "https://github.com/Davier/bevy_debug_menu", features = ["extra"] }
[patch.crates-io]
bevy = { git = "https://github.com/Davier/bevy", branch = "extra" }
```

## Warnings

 
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

* stable order for all items
* search by resource type, component type, entity Name and entity Label
* remove resource, entity, component
* load/save scene
* list and spawn loaded scenes
* integration with bevy_mod_picking