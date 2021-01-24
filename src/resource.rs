use bevy::{prelude::*, ui};

#[cfg(feature = "extra")]
use bevy::reflect::{ReflectResource, TypeRegistry};

use crate::widgets::ecr_tree;
use crate::DebugIgnore;

#[cfg(feature = "extra")]
#[derive(Reflect, Default)]
#[reflect(Resource)]
pub struct TestResource {
    string: String,
    int: usize,
    bool: bool,
}

#[derive(Debug)]
pub struct ResourceList;

pub fn spawn(commands: &mut Commands, style: &ecr_tree::Style) -> Entity {
    let mut entity = None;
    commands.with_children(|parent| {
        parent
            .spawn(NodeBundle {
                style: ui::Style {
                    flex_direction: FlexDirection::ColumnReverse,
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
            .with(ResourceList)
            .with(DebugIgnore)
            .with_children(|_parent| {
                #[cfg(not(feature = "extra"))]
                _parent.spawn(TextBundle {
                    text: Text::with_section(
                        "cargo feature \"extra\" is required",
                        TextStyle {
                            font: style.font.clone(),
                            font_size: 30.0,
                            color: Color::BLACK,
                        },
                        Default::default(),
                    ),
                    style: ui::Style {
                        align_self: AlignSelf::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                });
            });
        entity = parent.current_entity();
        parent.with(ecr_tree::State::new(entity.unwrap(), style.clone()));
    });
    entity.unwrap()
}

#[cfg(feature = "extra")]
pub fn update_system(world: &mut World, resources: &mut Resources) {
    use std::ops::Deref;
    let type_registry_arc = resources.get::<TypeRegistry>().unwrap().deref().clone();
    let type_registry = type_registry_arc.read();
    let mut unregistered_count = 0;
    let resource_list: Vec<ecr_tree::Key> = resources
        .iter_types()
        .filter_map(|&type_id| {
            if type_registry.get(type_id).is_some() {
                Some(ecr_tree::Key::Resource { type_id })
            } else {
                unregistered_count += 1;
                None
            }
        })
        .collect();
    // TODO: Show count of unregistered resources

    for mut state in world.query_filtered_mut::<&mut ecr_tree::State, With<ResourceList>>() {
        let keys = state.get_root_keys_mut();
        keys.clear();
        keys.append(&mut resource_list.clone());
    }
}
