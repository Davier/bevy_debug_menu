use bevy::{
    ecs::Res,
    math::{Rect, Size},
    prelude::{trace, warn, BuildChildren, ChildBuilder, EventReader, Query, TextBundle},
    reflect::serde::{ReflectDeserializer, ReflectSerializer},
    text::{Text, TextStyle},
    ui::{self, AlignSelf, Val},
};
use serde::de::DeserializeSeed;

use crate::{
    widgets::{check_box::BuildCheckBox, tree_node::BuildTreeNode},
    DebugIgnore,
};

use self::input_box::BuildInputBox;

use super::*;

pub fn visit_reflect_leaf(
    spawner: &FnSpawnWidget,
    commands: &mut Commands,
    state: &mut State,
    _type_registry_arc: TypeRegistry,
    reflect: &mut dyn Reflect,
    name: String,
    container: Entity,
) -> bool {
    // Insert
    let key = Key::ReflectLeaf {
        address: reflect as *mut dyn Reflect as *mut () as usize,
        type_id: reflect.type_id(),
    };
    let entry = {
        if let Some(entry) = state.entries.get_mut(&key) {
            entry
        } else {
            let widget = spawner(
                key,
                commands,
                state,
                _type_registry_arc,
                reflect,
                name,
                container,
            );
            state.entries.insert(
                key,
                Entry {
                    widget,
                    inner: EntryType::Leaf {
                        value: reflect.clone_value(),
                        field_mutated: true,
                        widget_mutated: None,
                    },
                },
            );
            state.entries.get_mut(&key).unwrap()
        }
    };
    state.entries_alive.insert(key, true);

    // Update
    let mut mutated = false;
    if let EntryType::Leaf {
        value,
        field_mutated,
        widget_mutated,
    } = &mut entry.inner
    {
        if let Some(widget_mutated) = widget_mutated.take() {
            trace!("Setting value from widget");
            reflect.set(widget_mutated).unwrap();
            value.set(reflect.clone_value()).unwrap();
            mutated = true;
        } else if !reflect.reflect_partial_eq(value.as_ref()).unwrap_or(false) {
            trace!("Field has changed");
            value.set(reflect.clone_value()).unwrap();
            *field_mutated = true;
        }
    } else {
        unreachable!();
    }
    mutated
}

pub fn spawn_widget_default(
    key: super::Key,
    commands: &mut Commands,
    state: &mut super::State,
    _type_registry_arc: TypeRegistry,
    _reflect: &mut dyn Reflect,
    _name: String,
    container: Entity,
) -> Entity {
    commands.set_current_entity(container);
    let inputbox = commands.spawn_input_box(
        state.style.style_input_box.clone(),
        Some(super::with_debug_ignore),
    );
    commands.insert_one(
        inputbox.widget,
        EntryAccess {
            state_entity: state.state_entity.unwrap(),
            key,
        },
    );
    inputbox.widget
}

pub fn spawn_widget_bool(
    key: Key,
    commands: &mut Commands,
    state: &mut State,
    _type_registry_arc: TypeRegistry,
    reflect: &mut dyn Reflect,
    name: String,
    container: Entity,
) -> Entity {
    commands.set_current_entity(container);
    let tree_node = commands.spawn_tree_node(
        state.style.style_node.clone(),
        Some(|parent: &mut ChildBuilder| {
            parent.with(DebugIgnore);
        }),
    );
    // Override button behaviour, we only use the same style
    commands.remove_one::<tree_node::Button>(tree_node.button);

    commands.set_current_entity(tree_node.button);
    commands.with_children(|parent| {
        parent
            .spawn(TextBundle {
                style: ui::Style {
                    align_self: AlignSelf::Center,
                    size: Size {
                        width: Val::Undefined,
                        height: Val::Px(20.),
                    },
                    flex_shrink: 1.,
                    flex_grow: 1.,
                    margin: Rect {
                        left: Val::Px(26.0),
                        right: Val::Px(5.0),
                        top: Val::Px(5.0),
                        bottom: Val::Px(5.0),
                    },
                    ..Default::default()
                },
                text: Text::with_section(
                    format!("{}bool", name),
                    TextStyle {
                        font: state.style.font.clone(),
                        font_size: 20.0,
                        color: state.style.color_node_text,
                    },
                    Default::default(),
                ),
                ..Default::default()
            })
            .with(DebugIgnore)
            .current_entity();
    });
    let checked = *reflect.downcast_ref().unwrap();
    let checkbox = commands.spawn_check_box(
        checked,
        state.style.style_check_box.clone(),
        Some(with_debug_ignore),
    );
    commands.insert_one(
        checkbox.widget,
        EntryAccess {
            state_entity: state.state_entity.unwrap(),
            key,
        },
    );
    tree_node.widget
}

pub fn update_checkbox_system(
    mut query_checkbox: Query<(&mut check_box::Widget, &EntryAccess)>,
    mut query_state: Query<&mut State>,
    mut checkbox_event: EventReader<check_box::ToggledEvent>,
) {
    // Propagate widget event to state
    for event in checkbox_event.iter() {
        if let Ok((_checkbox, access)) = query_checkbox.get_mut(event.entity) {
            if let Ok(mut state) = query_state.get_mut(access.state_entity) {
                if let Some(entry) = state.entries.get_mut(&access.key) {
                    if let EntryType::Leaf { widget_mutated, .. } = &mut entry.inner {
                        trace!("Propagating widget event");
                        *widget_mutated = Some(Box::new(event.checked));
                    } else {
                        warn!("Invalid acces");
                    }
                } else {
                    warn!("Invalid acces");
                }
            } else {
                warn!("Invalid acces");
            }
        } else {
            warn!("Invalid acces");
        }
    }
    // Propagate field change to widget
    for (mut checkbox, access) in query_checkbox.iter_mut() {
        if let Ok(mut state) = query_state.get_mut(access.state_entity) {
            if let Some(entry) = state.entries.get_mut(&access.key) {
                if let EntryType::Leaf {
                    value,
                    field_mutated,
                    ..
                } = &mut entry.inner
                {
                    if *field_mutated {
                        trace!("Propagating field mutation");
                        checkbox.set(*value.downcast_ref().unwrap());
                        *field_mutated = false;
                    }
                } else {
                    warn!("Invalid acces");
                }
            } else {
                warn!("Invalid acces");
            }
        } else {
            warn!("Invalid acces");
        }
    }
}

pub fn update_inputbox_system(
    mut query_inputbox: Query<(&input_box::Widget, &EntryAccess)>,
    mut query_cursor: Query<(&input_box::Cursor, &mut Text)>,
    mut query_state: Query<&mut State>,
    mut inputbox_event: EventReader<input_box::UnfocusedEvent>,
    type_registry_arc: Res<TypeRegistry>,
) {
    // Propagate widget event to state
    for event in inputbox_event.iter() {
        if let Ok((_inputbox, access)) = query_inputbox.get_mut(event.entity) {
            if let Ok(mut state) = query_state.get_mut(access.state_entity) {
                if let Some(entry) = state.entries.get_mut(&access.key) {
                    if let EntryType::Leaf {
                        widget_mutated,
                        value,
                        field_mutated,
                    } = &mut entry.inner
                    {
                        trace!("Propagating widget event");
                        if event.canceled {
                            // Reset to correct text
                            *field_mutated = true;
                        } else {
                            // Deserialize edited text
                            let type_registry = type_registry_arc.read();
                            let serializer = ReflectSerializer::new(value.as_ref(), &type_registry);
                            let serialized_value = ron::ser::to_string(&serializer)
                                .unwrap_or_else(|e| format!("Serialization failed: {}", e));
                            let wrapped_edited_text =
                                wrap_serialized_value(serialized_value, &event.text);
                            let mut deserializer =
                                ron::de::Deserializer::from_str(&wrapped_edited_text).unwrap();
                            let reflect_deserializer = ReflectDeserializer::new(&type_registry);
                            match reflect_deserializer.deserialize(&mut deserializer) {
                                Ok(value) => {
                                    *widget_mutated = Some(value);
                                }
                                Err(e) => {
                                    warn!("Could not deserialize field: {}", e);
                                }
                            }
                        }
                    } else {
                        warn!("Invalid acces");
                    }
                } else {
                    warn!("Invalid acces");
                }
            } else {
                warn!("Invalid acces");
            }
        } else {
            warn!("Invalid acces");
        }
    }
    // Propagate field change to widget
    for (inputbox, access) in query_inputbox.iter() {
        if let Ok(mut state) = query_state.get_mut(access.state_entity) {
            if let Some(entry) = state.entries.get_mut(&access.key) {
                if let EntryType::Leaf {
                    value,
                    field_mutated,
                    ..
                } = &mut entry.inner
                {
                    if *field_mutated {
                        let (cursor, mut text) = query_cursor.get_mut(inputbox.text).unwrap();
                        if !cursor.is_focused() {
                            trace!("Propagating field mutation");
                            // Serialize value
                            let type_registry = type_registry_arc.read();
                            let serializer = ReflectSerializer::new(value.as_ref(), &type_registry);
                            let serialized_value = ron::ser::to_string(&serializer)
                                .unwrap_or_else(|e| format!("Serialization failed: {}", e));
                            text.sections[0].value.clear();
                            text.sections[0]
                                .value
                                .push_str(extract_serialized_value(&serialized_value));
                            *field_mutated = false;
                        }
                    }
                } else {
                    warn!("Invalid acces");
                }
            } else {
                warn!("Invalid acces");
            }
        } else {
            warn!("Invalid acces");
        }
    }
}

fn wrap_serialized_value(mut serialized_text: String, edited_text: &str) -> String {
    let value = "\"value\":";
    let start_index = serialized_text.find(value).unwrap() + value.len();
    let end_index = serialized_text.len() - 1;
    serialized_text.replace_range(start_index..end_index, edited_text);
    serialized_text
}

fn extract_serialized_value(serialized_text: &str) -> &str {
    let value = "\"value\":";
    let start_index = serialized_text.find(value).unwrap() + value.len();
    let end_index = serialized_text.len() - 1;
    &serialized_text[start_index..end_index]
}
