use bevy::{prelude::*, ui};

use crate::widgets::ecr_tree;
use crate::DebugIgnore;

#[derive(Debug)]
pub struct EntityList;

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
            .with(EntityList)
            .with(DebugIgnore);
        entity = parent.current_entity();
        parent.with(ecr_tree::State::new(entity.unwrap(), style.clone()));
    });
    entity.unwrap()
}

pub fn update_system(world: &mut World, _resources: &mut Resources) {
    let debugged_entities = world
        .query_filtered::<Entity, Without<DebugIgnore>>()
        .map(|entity| ecr_tree::Key::Entity { entity })
        .collect::<Vec<_>>();

    for mut state in world.query_filtered_mut::<&mut ecr_tree::State, With<EntityList>>() {
        let keys = state.get_root_keys_mut();
        keys.clear();
        keys.append(&mut debugged_entities.clone());
    }
}
