use std::ops::Deref;

#[cfg(feature = "extra")]
use bevy::reflect::ReflectResource;
use bevy::{
    core::{Labels, Name},
    ecs::{ComponentFlags, TypeInfo, With},
    prelude::{
        trace, Added, Commands, DespawnRecursiveExt, Entity, Mutated, Or, Query, ReflectComponent,
        Resources, World,
    },
    reflect::TypeRegistry,
    text::Text,
};
#[cfg(feature = "extra")]
use std::any::TypeId;

pub fn update_system(world: &mut World, resources: &mut Resources) {
    let mut commands = Commands::default();
    commands.set_entity_reserver(world.get_entity_reserver());

    let entities = world
        .query_filtered::<Entity, With<super::State>>()
        .collect::<Vec<_>>();
    for container in entities {
        // Take ownership of the state so that world is not borrowed during the update
        let mut state = std::mem::take(&mut *world.get_mut::<super::State>(container).unwrap());

        // Mark all tree nodes as dead
        for (_, is_alive) in state.entries_alive.iter_mut() {
            *is_alive = false;
        }

        commands.set_current_entity(container);

        // Visit all root keys
        let keys = std::mem::take(&mut state.root_keys);
        for &key in keys.iter() {
            match key {
                #[cfg(feature = "extra")]
                super::Key::Resource { type_id } => {
                    let type_registry_arc =
                        resources.get::<TypeRegistry>().unwrap().deref().clone();
                    visit_resource(
                        &mut commands,
                        &mut state,
                        resources,
                        type_registry_arc,
                        type_id,
                        container,
                    );
                }
                super::Key::Entity { entity } => {
                    let type_registry_arc =
                        resources.get::<TypeRegistry>().unwrap().deref().clone();
                    visit_entity(
                        &mut commands,
                        &mut state,
                        world,
                        type_registry_arc,
                        entity,
                        container,
                    );
                }
                super::Key::Component { entity, type_id } => {
                    let entity_location = world.get_entity_location(entity).unwrap();
                    let type_info = *world.archetypes[entity_location.archetype as usize]
                        .types()
                        .iter()
                        .find(|type_info| type_info.id() == type_id)
                        .unwrap();
                    let type_registry_arc =
                        resources.get::<TypeRegistry>().unwrap().deref().clone();
                    visit_component(
                        &mut commands,
                        &mut state,
                        world,
                        type_registry_arc,
                        entity,
                        type_info,
                        container,
                    );
                }
                super::Key::ReflectNode { .. } => {
                    unimplemented!();
                }
                super::Key::ReflectLeaf { .. } => {
                    unimplemented!();
                }
            }
        }
        state.root_keys = keys;

        // Delete all entries that were not visited
        let entries_alive = &mut state.entries_alive;
        let entries = &mut state.entries;
        entries_alive.retain(|key, is_alive| {
            if !*is_alive {
                trace!("removing node: {:?}", key);
                let entry = entries.remove(key);
                if let Some(entry) = entry {
                    commands.despawn_recursive(entry.widget);
                }
            }
            *is_alive
        });

        // Put back the state in the component
        *world.get_mut::<super::State>(container).unwrap() = state;
    }

    // Apply changes
    commands.apply(world, resources);
}

#[cfg(feature = "extra")]
fn visit_resource(
    commands: &mut Commands,
    state: &mut super::State,
    resources: &mut Resources,
    type_registry_arc: TypeRegistry,
    type_id: TypeId,
    container: Entity,
) {
    let key = super::Key::Resource { type_id };
    let entry = {
        if let Some(entry) = state.entries.get_mut(&key) {
            entry
        } else {
            let type_registry = type_registry_arc.read();
            let label = if let Some(registration) = type_registry.get(type_id) {
                if registration.data::<ReflectResource>().is_some() {
                    registration.short_name().to_string()
                } else {
                    format!("{} (not a ReflectResource)", registration.short_name())
                }
            } else {
                "(not a registered type)".to_string()
            };
            let node = super::node::spawn_widget_node(key, commands, state, label, container);
            state.entries.insert(
                key,
                super::Entry {
                    widget: node.root,
                    inner: super::EntryType::Node { container: None },
                },
            );
            state.entries.get_mut(&key).unwrap()
        }
    };
    state.entries_alive.insert(key, true);

    if let super::EntryType::Node { container, .. } = entry.inner {
        if let Some(container) = container {
            let type_registry = type_registry_arc.read();
            if let Some(registration) = type_registry.get(type_id) {
                if let Some(reflect_resource) = registration.data::<ReflectResource>().cloned() {
                    drop(type_registry);
                    reflect_resource.borrow_mut_resource(resources);
                    // Safety:
                    //      we just borrowed the reflected resource exclusively
                    //      resources is not used to access the reflected resource until it is released
                    let resource = unsafe { reflect_resource.reflect_resource_mut(resources) };
                    let mutated = super::node::dispatch_reflect(
                        commands,
                        state,
                        type_registry_arc,
                        resource,
                        container,
                    );
                    if mutated {
                        trace!("Resource mutated: {}", resource.type_name());
                        resources.set_mutated_dynamic(&type_id);
                    }
                    // Safety: the reflected resource was borrowed in this function
                    unsafe {
                        reflect_resource.release_mut_resource(resources);
                    }
                }
            }
        }
    } else {
        unreachable!();
    }
}

fn visit_entity(
    commands: &mut Commands,
    state: &mut super::State,
    world: &mut World,
    type_registry_arc: TypeRegistry,
    entity: Entity,
    container: Entity,
) {
    let key = super::Key::Entity { entity };
    let entry = {
        if let Some(entry) = state.entries.get_mut(&key) {
            entry
        } else {
            let node =
                super::node::spawn_widget_node(key, commands, state, String::new(), container);
            commands.insert_one(node.label, EntityLabel { target: entity });
            state.entries.insert(
                key,
                super::Entry {
                    widget: node.root,
                    inner: super::EntryType::Node { container: None },
                },
            );
            state.entries.get_mut(&key).unwrap()
        }
    };
    state.entries_alive.insert(key, true);

    if let super::EntryType::Node { container, .. } = entry.inner {
        if let Some(container) = container {
            let entity_location = world.get_entity_location(entity).unwrap();
            let component_types =
                Vec::from(world.archetypes[entity_location.archetype as usize].types());
            for &type_info in component_types.iter() {
                visit_component(
                    commands,
                    state,
                    world,
                    type_registry_arc.clone(),
                    entity,
                    type_info,
                    container,
                );
            }
        }
    } else {
        unreachable!();
    }
}

fn visit_component(
    commands: &mut Commands,
    state: &mut super::State,
    world: &mut World,
    type_registry_arc: TypeRegistry,
    entity: Entity,
    type_info: TypeInfo,
    container: Entity,
) {
    let type_id = type_info.id();
    let key = super::Key::Component { entity, type_id };
    let entry = {
        if let Some(entry) = state.entries.get_mut(&key) {
            entry
        } else {
            let type_registry = type_registry_arc.read();
            let label = if let Some(registration) = type_registry.get(type_id) {
                if registration.data::<ReflectComponent>().is_some() {
                    registration.short_name().to_string()
                } else {
                    format!("{} (not a ReflectComponent", registration.short_name())
                }
            } else {
                format!("{} (not a registered type)", type_info.type_name())
            };
            let node = super::node::spawn_widget_node(key, commands, state, label, container);
            state.entries.insert(
                key,
                super::Entry {
                    widget: node.root,
                    inner: super::EntryType::Node { container: None },
                },
            );
            state.entries.get_mut(&key).unwrap()
        }
    };
    state.entries_alive.insert(key, true);

    if let super::EntryType::Node { container, .. } = entry.inner {
        if let Some(container) = container {
            let type_registry = type_registry_arc.read();
            if let Some(registration) = type_registry.get(type_id) {
                if let Some(reflect_component) = registration.data::<ReflectComponent>() {
                    let entity_location = world.get_entity_location(entity).unwrap();
                    let entity_archetype =
                        &mut world.archetypes[entity_location.archetype as usize];
                    // SAFETY:
                    //      we just obtained entity_archetype and entity_location from world so they are valid
                    //      world and entity_archetype are not used to access the reflected component until it is dropped
                    let component = unsafe {
                        reflect_component
                            .reflect_component_mut(entity_archetype, entity_location.index)
                    };
                    drop(type_registry);
                    let mutated = super::node::dispatch_reflect(
                        commands,
                        state,
                        type_registry_arc,
                        component,
                        container,
                    );
                    if mutated {
                        trace!("Component mutated: {:?}::{}", entity, component.type_name());
                        let component_state = entity_archetype.get_type_state_mut(type_id).unwrap();
                        // Safety: world is borrowed exclusively
                        unsafe {
                            component_state
                                .component_flags()
                                .as_mut()
                                .insert(ComponentFlags::MUTATED);
                        }
                    }
                }
            }
        }
    } else {
        unreachable!();
    }
}

pub struct EntityLabel {
    target: Entity,
}

pub fn update_entity_labels(
    mut query_label: Query<(Entity, &mut Text, &EntityLabel)>,
    query_added: Query<(), Added<EntityLabel>>,
    query_mutated: Query<(), Or<(Mutated<Name>, Mutated<Labels>)>>,
    query_entity: Query<(Option<&Name>, Option<&Labels>)>,
) {
    for (entity, mut text, entity_label) in query_label.iter_mut() {
        if query_added.get(entity).is_ok() || query_mutated.get(entity_label.target).is_ok() {
            let (name, labels) = query_entity.get(entity_label.target).unwrap();
            let value = &mut text.sections[0].value;
            *value = format!("{:?}", entity_label.target);
            if let Some(name) = name {
                value.push(' ');
                value.push_str(name.as_str())
            }
            if let Some(labels) = labels {
                value.push_str(" [");
                for (i, entity_label) in labels.iter().enumerate() {
                    if i > 0 {
                        value.push_str(", ");
                    }
                    value.push_str(entity_label);
                }
                value.push(']');
            }
        }
    }
}
