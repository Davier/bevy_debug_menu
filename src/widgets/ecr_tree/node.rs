use bevy::{
    ecs::With,
    math::{Rect, Size},
    prelude::{
        trace, warn, BuildChildren, ColorMaterial, Commands, Entity, Handle, ImageBundle, Mutated,
        Query, Reflect, TextBundle,
    },
    reflect::TypeRegistry,
    text::{Text, TextStyle},
    ui::{self, AlignSelf, Val},
};

use crate::{
    widgets::tree_node::{self, BuildTreeNode},
    DebugIgnore,
};

pub struct ExpandIcon {
    widget: Entity,
    icon_chevron_down: Handle<ColorMaterial>,
    icon_chevron_up: Handle<ColorMaterial>,
}

pub fn dispatch_reflect(
    commands: &mut Commands,
    state: &mut super::State,
    type_registry_arc: TypeRegistry,
    reflect: &mut dyn Reflect,
    container: Entity,
) -> bool {
    let mut mutated = false;
    match reflect.reflect_mut() {
        bevy::reflect::ReflectMut::Struct(s) => {
            for i in 0..s.field_len() {
                let name = format!("{}: ", s.name_at(i).unwrap());
                mutated |= super::node::visit_reflect_node(
                    commands,
                    state,
                    type_registry_arc.clone(),
                    s.field_at_mut(i).unwrap(),
                    name,
                    None,
                    container,
                );
            }
        }
        bevy::reflect::ReflectMut::Tuple(t) => {
            for i in 0..t.field_len() {
                let name = format!("{}: ", i);
                mutated |= super::node::visit_reflect_node(
                    commands,
                    state,
                    type_registry_arc.clone(),
                    t.field_mut(i).unwrap(),
                    name,
                    None,
                    container,
                );
            }
        }
        bevy::reflect::ReflectMut::TupleStruct(ts) => {
            for i in 0..ts.field_len() {
                let name = format!("{}: ", i);
                mutated |= super::node::visit_reflect_node(
                    commands,
                    state,
                    type_registry_arc.clone(),
                    ts.field_mut(i).unwrap(),
                    name,
                    None,
                    container,
                );
            }
        }
        bevy::reflect::ReflectMut::List(l) => {
            for i in 0..l.len() {
                let name = format!("[{}]:", i);
                mutated |= super::node::visit_reflect_node(
                    commands,
                    state,
                    type_registry_arc.clone(),
                    l.get_mut(i).unwrap(),
                    name,
                    None,
                    container,
                );
            }
        }
        bevy::reflect::ReflectMut::Map(m) => {
            for i in 0..m.len() {
                let key = m.get_at(i).unwrap().0.clone_value();
                let name = format!(
                    "{}: ",
                    super::serialize_reflect(&*key).unwrap_or_else(|| i.to_string())
                );
                let value = m.get_mut(&*key).unwrap();
                mutated |= super::node::visit_reflect_node(
                    commands,
                    state,
                    type_registry_arc.clone(),
                    value,
                    name,
                    None,
                    container,
                );
            }
        }
        bevy::reflect::ReflectMut::Value(v) => {
            mutated |= super::leaf::visit_reflect_leaf(
                &(super::leaf::spawn_widget_default as super::FnSpawnWidget),
                commands,
                state,
                type_registry_arc,
                v,
                "".to_string(),
                None,
                container,
            );
        }
        #[cfg(feature = "enum")]
        bevy::reflect::ReflectMut::Enum(e) => {
            let index = e.variant_info().index;
            for i in 0..e.iter_variants_info().count() {
                let variant_name = e.get_index_name(i).unwrap().to_string();
                if i == index {
                    let (name, value) = match e.variant_mut() {
                        bevy::reflect::EnumVariantMut::Unit => {
                            mutated |= super::leaf::visit_reflect_leaf(
                                &(super::leaf::spawn_widget_unit_active as super::FnSpawnWidget),
                                commands,
                                state,
                                type_registry_arc.clone(),
                                e.as_reflect_mut(), // This will not be used
                                variant_name,
                                Some(i),
                                container,
                            );
                            continue;
                        }
                        // bevy::reflect::EnumVariantMut::NewType(t) => (format!("{}({})", variant_name, t.type_name()), t),
                        bevy::reflect::EnumVariantMut::NewType(t) => {
                            (format!("{}(", variant_name), t)
                        }
                        bevy::reflect::EnumVariantMut::Tuple(t) => (
                            format!("{}{}", variant_name, t.type_name()),
                            t.as_reflect_mut(),
                        ),
                        bevy::reflect::EnumVariantMut::Struct(s) => (
                            format!("{} {}", variant_name, s.type_name()),
                            s.as_reflect_mut(),
                        ),
                    };
                    mutated |= super::node::visit_reflect_node(
                        commands,
                        state,
                        type_registry_arc.clone(),
                        value,
                        name, // FIXME
                        Some(i),
                        container,
                    );
                } else {
                    super::leaf::visit_reflect_leaf(
                        &(super::leaf::spawn_widget_wrong_variant as super::FnSpawnWidget),
                        commands,
                        state,
                        type_registry_arc.clone(),
                        e.as_reflect_mut(),
                        variant_name,
                        Some(i),
                        container,
                    );
                }
            }
        }
    }
    mutated
}

pub fn visit_reflect_node(
    commands: &mut Commands,
    state: &mut super::State,
    type_registry_arc: TypeRegistry,
    reflect: &mut dyn Reflect,
    name: String,
    variant_index: Option<usize>,
    container: Entity,
) -> bool {
    // Hook for specialized widgets
    if let Some(spawner) = state.specialized_widgets.get(&reflect.type_id()).copied() {
        return super::leaf::visit_reflect_leaf(
            &spawner,
            commands,
            state,
            type_registry_arc,
            reflect,
            name,
            None,
            container,
        );
    }

    // Generic node widget
    let mut mutated = false;
    let key = super::Key::ReflectNode {
        address: reflect as *mut dyn Reflect as *mut () as usize,
        type_id: reflect.type_id(),
        variant_index,
    };
    let entry = {
        if let Some(entry) = state.entries.get_mut(&key) {
            entry
        } else {
            let type_registry = type_registry_arc.read();
            let label = if let Some(registration) = type_registry.get(reflect.type_id()) {
                format!("{}{}", name, registration.short_name())
            } else {
                format!("{}{} (unregistered)", name, reflect.type_name())
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
            mutated |= dispatch_reflect(commands, state, type_registry_arc, reflect, container);
        }
    } else {
        unreachable!();
    }
    mutated
}

pub struct NodeBuilder {
    pub root: Entity,
    pub label: Entity,
}

pub fn spawn_widget_node(
    key: super::Key,
    commands: &mut Commands,
    state: &mut super::State,
    name: String,
    container: Entity,
) -> NodeBuilder {
    commands.set_current_entity(container);
    let tree_node = commands.spawn_tree_node(
        state.style.style_node.clone(),
        Some(super::with_debug_ignore),
    );
    commands.insert_one(
        tree_node.widget,
        super::EntryAccess {
            state_entity: state.state_entity.unwrap(),
            key,
        },
    );
    let mut label = None;
    let mut expand_icon = None;
    commands.set_current_entity(tree_node.button);
    commands.with_children(|parent| {
        expand_icon = parent
            .spawn(ImageBundle {
                style: ui::Style {
                    align_self: AlignSelf::Center,
                    margin: Rect::all(Val::Px(5.0)),
                    flex_shrink: 0.,
                    size: Size {
                        width: Val::Px(16.0),
                        height: Val::Px(16.0),
                    },
                    ..Default::default()
                },
                material: state.style.icon_chevron_down.clone(),
                ..Default::default()
            })
            .with(ExpandIcon {
                widget: tree_node.widget,
                icon_chevron_down: state.style.icon_chevron_down.clone(),
                icon_chevron_up: state.style.icon_chevron_up.clone(),
            })
            .with(DebugIgnore)
            .current_entity();
        label = parent
            .spawn(TextBundle {
                style: ui::Style {
                    align_self: AlignSelf::Center,
                    size: Size {
                        width: Val::Undefined,
                        height: Val::Px(20.),
                    },
                    flex_shrink: 0.,
                    margin: Rect {
                        left: Val::Px(0.0),
                        right: Val::Px(5.0),
                        top: Val::Px(5.0),
                        bottom: Val::Px(5.0),
                    },
                    ..Default::default()
                },
                text: Text::with_section(
                    name,
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
    NodeBuilder {
        root: tree_node.widget,
        label: label.unwrap(),
    }
}

pub fn update_node_system(
    mut query_icon: Query<(&ExpandIcon, &mut Handle<ColorMaterial>)>,
    query_node: Query<&tree_node::Widget, Mutated<tree_node::Widget>>,
    query_access: Query<&super::EntryAccess, With<tree_node::Widget>>,
    mut query_state: Query<&mut super::State>,
) {
    for access in query_access.iter() {
        if let Ok(mut state) = query_state.get_mut(access.state_entity) {
            if let Some(entry) = state.entries.get_mut(&access.key) {
                if let super::EntryType::Node { container, .. } = &mut entry.inner {
                    if let Ok(node) = query_node.get(entry.widget) {
                        trace!("Updating node children container for {:?}", entry.widget);
                        *container = node.get_children_container();
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
    for (icon, mut material) in query_icon.iter_mut() {
        if query_node.get(icon.widget).is_ok() {
            trace!("Updating node expand icon");
            let container = query_node
                .get(icon.widget)
                .map(|node| node.get_children_container())
                .unwrap_or(None);
            if container.is_some() {
                *material = icon.icon_chevron_up.clone();
            } else {
                *material = icon.icon_chevron_down.clone();
            }
        }
    }
}
