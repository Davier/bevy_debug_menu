use std::io::Write;

use bevy::{
    prelude::*,
    reflect::TypeRegistry,
    ui::{self, FocusPolicy},
};

use crate::DebugIgnore;

#[derive(Debug)]
pub struct SceneList {
    style: Style,
}

#[derive(Debug, Clone)]
pub struct Style {
    pub font: Handle<Font>,
    pub font_size: f32,
    pub color_background: Handle<ColorMaterial>,
}

pub struct SaveSceneButton;

pub fn spawn(commands: &mut Commands, style: &Style) -> Entity {
    let mut entity = None;
    commands.with_children(|parent| {
        parent
            .spawn(NodeBundle {
                style: ui::Style {
                    flex_direction: FlexDirection::ColumnReverse,
                    position: Rect {
                        left: Val::Undefined,
                        top: Val::Px(0.0), // We use this for vertical scrolling
                        bottom: Val::Undefined,
                        right: Val::Undefined,
                    },
                    size: Size {
                        width: Val::Percent(100.),
                        height: Val::Undefined, // Height will grow as needed
                    },
                    flex_shrink: 0.,
                    padding: Rect {
                        left: Val::Px(4.0),
                        right: Val::Px(4.0),
                        top: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                    },
                    ..Default::default()
                },
                material: style.color_background.clone(),
                ..Default::default()
            })
            .with(Children::default())
            .with(SceneList {
                style: style.clone(),
            })
            .with(DebugIgnore)
            .with_children(|parent| {
                parent
                    .spawn(ButtonBundle::default())
                    .with(SaveSceneButton)
                    .with(DebugIgnore)
                    .with_children(|parent| {
                        parent
                            .spawn(TextBundle {
                                text: Text::with_section(
                                    "Save",
                                    TextStyle {
                                        font: style.font.clone(),
                                        font_size: style.font_size,
                                        color: Color::BLACK,
                                    },
                                    TextAlignment::default(),
                                ),
                                focus_policy: FocusPolicy::Pass,
                                ..Default::default()
                            })
                            .with(DebugIgnore);
                    });
            });
        entity = Some(parent.current_entity().unwrap());
    });
    entity.unwrap()
}

pub fn interact_save_button(world: &mut World, resources: &mut Resources) {
    for interaction in world.query_filtered::<&Interaction, With<SaveSceneButton>>() {
        if *interaction == Interaction::Clicked {
            let type_registry = resources.get::<TypeRegistry>().unwrap();
            let scene = DynamicScene::from_world(&world, &type_registry);
            let mut file = std::fs::File::create("assets/scenes/output.scn").unwrap();
            write!(file, "{}", scene.serialize_ron(&type_registry).unwrap()).unwrap();
        }
    }
}
