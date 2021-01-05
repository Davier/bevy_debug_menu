# Bevy Debug Menu
 
This crate adds a debug menu to your [Bevy](https://bevyengine.org) game that can show and edit all the current entities.
 
Usage:

* add the [DebugMenuPlugin] to your bevy app
* add the [DebugMenuFont] resource (and the font file to your `assets/` folder)
* optional: adapt your game to stop processing keyboard inputs by listening to [FocusedEvent] and [UnfocusedEvent]
* press F10 to show or hide the menu
 
## Warnings

You need to compile in release mode to have descent frame rate. 

This contains unsafe code that needs reviewing.
 
The debug menu will push the rest of your UI elements even when hidden. A PR is being made to bevy to fix that.

This is a prototype.
 
## Example

```
use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(bevy_debug_menu::DebugMenuPlugin)
        .add_resource(bevy_debug_menu::DebugMenuFont{
            path: "your_font.ttf"
        })
        // This is unnecessary if you already initialize the UI camera in your game
        .add_startup_system(bevy_debug_menu::setup_ui_camera.system())
        .run();
}
```

## TODO

* filtering for components
* remove entities/components
* improve the edit box widget
* always draw over other UI elements
* specialized widgets to edit some types (e.g. bool, ints, floats)
* implement reflection for enums
* integration with bevy_mod_picking

