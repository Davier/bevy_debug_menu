use bevy::{
    ecs::{Commands, ComponentFlags},
    input::mouse::MouseWheel,
    prelude::*,
    reflect::{
        serde::{ReflectDeserializer, ReflectSerializer},
        TypeRegistry, TypeRegistryArc,
    },
    utils::HashMap,
};
use serde::de::DeserializeSeed;

use std::{any::TypeId, ops::Deref};

use crate::tree_node::{self, BuildTreeNode, TreeNode};
use crate::{edit_box, tree_node::TreeNodeStyle};

// Does not panic when min > max unlike bevy::math::clamp
pub fn clamp<T: PartialOrd>(input: T, min: T, max: T) -> T {
    if input < min {
        min
    } else if input > max {
        max
    } else {
        input
    }
}

pub struct DebugMenuPlugin;

pub struct DebugMenuFont {
    pub path: &'static str,
}

impl Plugin for DebugMenuPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<UiState>()
            .init_resource::<UiStyle>()
            .add_event::<edit_box::UnfocusedEvent>()
            .add_event::<edit_box::FocusedEvent>()
            .add_system(edit_box::interact_mouse.system())
            .add_system(edit_box::interact_keyboard.system())
            .add_system(tree_node::interact_button::<(DebugIgnore,)>.system())
            .add_system(update_menu.system())
            .add_system(input.system())
            .add_system(update_labels.system())
            .add_system(unfocused_edit_box_event.system());
    }
}

// Marker component for the TextBundle inside tree nodes
pub struct NodeLabel {
    tree_node_root: Entity,
}

// Marker component to filter out all of the debug menu's entities
#[derive(Clone, Copy)]
pub struct DebugIgnore;

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub enum UiNodeTarget {
    Entity(DebuggedEntity),
    Component {
        entity: DebuggedEntity,
        type_id: TypeId,
        type_name: &'static str,
    },
    Field {
        entity: DebuggedEntity,
        component_type_id: TypeId,
        type_id: TypeId,
        field_id: FieldId,
    },
    EditBox {
        entity: DebuggedEntity,
        component_type_id: TypeId,
        parent_field_id: FieldId,
    },
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct FieldId(u64);

impl FieldId {
    fn new(field: &dyn Reflect) -> Self {
        let address = field as *const dyn Reflect as *const () as u64;
        Self(address)
    }
}

// Identify entities that are being debugged
#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct DebuggedEntity(Entity);
impl Deref for DebuggedEntity {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// Identify entities that are part of the debug menu
#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct UiEntity(Entity);
impl Deref for UiEntity {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default)]
pub struct UiState {
    root_debug_ui: Option<UiEntity>,
    map_ui_nodes: HashMap<UiNodeTarget, UiEntity>,
    map_ui_nodes_is_alive: HashMap<UiNodeTarget, bool>,
    updated_components: Vec<(DebuggedEntity, TypeId, UiEntity)>,
    scrolling_position: f32,
    show_progress: f32,
    show: bool,
}

pub struct UiStyle {
    pub style_entity: tree_node::TreeNodeStyle,
    pub style_component: tree_node::TreeNodeStyle,
    pub style_field: tree_node::TreeNodeStyle,
    pub color_background: Handle<ColorMaterial>,
    pub color_background_error: Handle<ColorMaterial>,
    pub font: Handle<Font>,
    pub color_title_text: Color,
    pub color_entity_text: Color,
}

impl FromResources for UiStyle {
    fn from_resources(resources: &Resources) -> Self {
        let mut materials = resources.get_mut::<Assets<ColorMaterial>>().unwrap();
        let asset_server = resources.get::<AssetServer>().unwrap();
        let font = resources.get::<DebugMenuFont>().expect("The DebugMenuFont must be created");

        let tree_node_style = TreeNodeStyle {
            color_root_container: materials.add(Color::rgba(0.2, 0.2, 0.2, 0.4).into()),
            color_button: materials.add(Color::BLACK.into()),
            color_button_hovered: Some(materials.add(Color::MIDNIGHT_BLUE.into())),
            color_button_clicked: Some(materials.add(Color::ALICE_BLUE.into())),
            color_button_expanded: Some(materials.add(Color::MIDNIGHT_BLUE.into())),
            color_children_container: materials.add(Color::rgba(0.2, 0.2, 0.2, 0.8).into()),
            node_style_button: Style {
                margin: Rect {
                    left: Val::Px(4.0),
                    right: Val::Px(4.0),
                    top: Val::Px(4.0),
                    bottom: Val::Px(0.0),
                },
                ..Default::default()
            },
            node_style_children_container: Style {
                flex_direction: FlexDirection::ColumnReverse,
                margin: Rect {
                    left: Val::Px(4.0),
                    right: Val::Px(4.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                },
                padding: Rect {
                    left: Val::Px(8.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                },
                ..Default::default()
            },
        };

        UiStyle {
            color_background: materials.add(Color::rgb(0.8, 0.8, 0.8).into()),
            color_background_error: materials.add(Color::RED.into()),
            font: asset_server.load(font.path),
            color_title_text: Color::BLACK,
            color_entity_text: Color::WHITE,
            style_entity: tree_node_style.clone(),
            style_component: tree_node_style.clone(),
            style_field: tree_node_style,
        }
    }
}

pub struct SerializedField(Option<String>);
pub struct EditedField(Option<String>);

// Recursively scan the entities, their components and their fields and update the debug menu
pub fn update_menu(world: &mut World, resources: &mut Resources) {
    // Own the UI state during its update so that `resources` is not borrowed
    let mut ui_state = std::mem::take(&mut *resources.get_mut::<UiState>().unwrap());

    // Update the main panel
    update_main_panel(world, resources, &mut ui_state);

    // Mark all tree nodes as dead
    for (_, is_alive) in ui_state.map_ui_nodes_is_alive.iter_mut() {
        *is_alive = false;
    }

    // Update every entity
    let mut commands = Commands::default();
    commands.set_entity_reserver(world.get_entity_reserver());
    commands.set_current_entity(*ui_state.root_debug_ui.unwrap());

    let debugged_entities = world
        .query_filtered::<Entity, Without<DebugIgnore>>()
        .collect::<Vec<_>>();
    for debugged_entity in debugged_entities.iter() {
        update_node(
            &mut commands,
            world,
            resources,
            &mut ui_state,
            UiNodeTarget::Entity(DebuggedEntity(*debugged_entity)),
            None,
            None,
        );
    }

    // Mark the updated components
    for (entity, component_type_id, edit_box) in ui_state.updated_components.drain(..) {
        let entity_location = world.get_entity_location(*entity).unwrap();
        let entity_archetype = &mut world.archetypes[entity_location.archetype as usize];
        let component_state = entity_archetype
            .get_type_state_mut(component_type_id)
            .unwrap();
        unsafe {
            component_state
                .component_flags()
                .as_mut()
                .insert(ComponentFlags::MUTATED);
        }
        if let Ok(mut edited_field) = world.get_mut::<EditedField>(*edit_box) {
            edited_field.0 = None;
        }
    }

    // Delete all tree node keys that were not visited
    let map_ui_nodes_is_alive = &mut ui_state.map_ui_nodes_is_alive;
    let map_ui_nodes = &mut ui_state.map_ui_nodes;
    map_ui_nodes_is_alive.retain(|target, is_alive| {
        if !*is_alive {
            trace!("removing node: {:?}", target);
            let entity = map_ui_nodes.remove(target);
            if let Some(entity) = entity {
                commands.despawn_recursive(*entity);
            }
        }
        *is_alive
    });

    // Move the state back to `resources`
    *resources.get_mut::<UiState>().unwrap() = ui_state;

    commands.apply(world, resources);
}

// Update the button labels to show if they are expanded
pub fn update_labels(
    query_node: Query<&TreeNode<(DebugIgnore,)>>,
    mut query_text: Query<(&mut Text, &NodeLabel)>,
) {
    for (mut text, update) in query_text.iter_mut() {
        let node = query_node.get(update.tree_node_root).unwrap();
        text.value
            .replace_range(0..1, if node.is_expanded() { "v" } else { ">" });
    }
}

fn update_main_panel(world: &mut World, resources: &mut Resources, ui_state: &mut UiState) {
    // Insert if needed
    if ui_state.root_debug_ui.is_none() {
        trace!("inserting main panel");

        let ui_style = resources.get::<UiStyle>().unwrap();
        let mut commands = Commands::default();
        commands.set_entity_reserver(world.get_entity_reserver());

        commands
            .spawn(NodeBundle {
                style: Style {
                    size: Size::new(Val::Percent(40.0), Val::Undefined),
                    min_size: Size::new(Val::Percent(40.0), Val::Percent(100.0)),
                    align_items: AlignItems::Stretch,
                    justify_content: JustifyContent::FlexStart,
                    flex_direction: FlexDirection::ColumnReverse,
                    position: Rect {
                        left: Val::Percent(0.0),
                        top: Val::Px(0.0),
                        ..Default::default()
                    },
                    align_self: AlignSelf::FlexEnd,
                    ..Default::default()
                },
                material: ui_style.color_background.clone(),
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
                ..Default::default()
            })
            .with(DebugIgnore)
            .with_children(|parent| {
                parent
                    .spawn(TextBundle {
                        style: Style {
                            align_self: AlignSelf::Center,
                            ..Default::default()
                        },
                        text: Text {
                            value: "Debug UI: entities".to_string(),
                            font: ui_style.font.clone(),
                            style: TextStyle {
                                font_size: 24.0,
                                color: ui_style.color_title_text,
                                ..Default::default()
                            },
                        },
                        ..Default::default()
                    })
                    .with(DebugIgnore);
            });
        ui_state.root_debug_ui = Some(UiEntity(commands.current_entity().unwrap()));
        // Drop all resources before commands.apply
        drop(ui_style);
        commands.apply(world, resources);
    }

    // Update
    if let Some(UiEntity(root)) = ui_state.root_debug_ui {
        let time = resources.get::<Time>().unwrap();
        let windows = resources.get_mut::<Windows>().unwrap();

        let window_height = windows.get_primary().unwrap().height();
        let panel_height = world.get_mut::<Node>(root).unwrap().size.y;
        if let Ok(mut style) = world.get_mut::<Style>(root) {
            // Vertical scrolling
            ui_state.scrolling_position = clamp(
                ui_state.scrolling_position,
                window_height - panel_height,
                0.0,
            );
            style.position.top = Val::Px(ui_state.scrolling_position);
            // Horizontal transition
            if ui_state.show_progress >= 0.0 {
                let transition_time = 0.2;
                ui_state.show_progress -= time.delta_seconds() / transition_time;
                let panel_width = match style.size.width {
                    Val::Percent(x) => x,
                    _ => unimplemented!(),
                };
                let (origin, target) = if ui_state.show {
                    (-panel_width, 0.0)
                } else {
                    (0.0, -panel_width)
                };
                use interpolation::*;
                style.position.left = Val::Percent(
                    origin + (1.0 - ui_state.show_progress).quadratic_in_out() * (target - origin),
                );
            }
        }
    }
}

fn update_node(
    commands: &mut Commands,
    world: &World,
    resources: &mut Resources,
    ui_state: &mut UiState,
    target: UiNodeTarget,
    reflect_field: Option<&dyn Reflect>,
    label_prefix: Option<String>,
) {
    let parent_container = commands.current_entity().unwrap();
    // Insert a new node if needed
    let node = ui_state.map_ui_nodes.entry(target).or_insert_with(|| {
        trace!("inserting new node: {:?}", target);

        // Generate the label
        let mut error = false;
        let type_registry = resources.get::<TypeRegistryArc>().unwrap();
        let type_registry_read = type_registry.read();
        let label = "> ".to_string()
            + label_prefix.unwrap_or("".to_string()).as_str()
            + match target {
                UiNodeTarget::Entity(entity) => format!("{:?}", *entity),
                UiNodeTarget::Component {
                    type_id, type_name, ..
                } => if let Some(registration) = type_registry_read.get(type_id) {
                    registration.short_name()
                } else {
                    error = true;
                    type_name
                }
                .to_string(),
                UiNodeTarget::Field { type_id, .. } => {
                    if let Some(registration) = type_registry_read.get(type_id) {
                        registration.short_name()
                    } else {
                        reflect_field.unwrap().type_name()
                    }
                    .to_string()
                }
                UiNodeTarget::EditBox { .. } => {
                    unreachable!();
                }
            }
            .as_str();

        // Add a new tree node with text inside
        let ui_style = resources.get::<UiStyle>().unwrap();
        let mut node_style = match target {
            UiNodeTarget::Entity(_) => &ui_style.style_entity,
            UiNodeTarget::Component { .. } => &ui_style.style_component,
            UiNodeTarget::Field { .. } => &ui_style.style_field,
            UiNodeTarget::EditBox { .. } => unreachable!(),
        }
        .clone();
        let mut text_style = Style {
            align_self: AlignSelf::Center,
            margin: Rect::all(Val::Px(5.0)),
            ..Default::default()
        };
        if error {
            node_style.color_button = ui_style.color_background_error.clone();
            text_style.align_self = AlignSelf::FlexEnd;
        }
        let root_entity = commands.spawn_tree_node(node_style, Some((DebugIgnore,)));
        commands.with_children(|parent| {
            parent
                .spawn(TextBundle {
                    style: text_style,
                    text: Text {
                        value: label,
                        font: ui_style.font.clone(),
                        style: TextStyle {
                            font_size: 20.0,
                            color: ui_style.color_entity_text,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with(NodeLabel {
                    tree_node_root: root_entity,
                })
                .with(DebugIgnore);
        });

        UiEntity(root_entity)
    });

    // Mark this node as alive
    ui_state.map_ui_nodes_is_alive.insert(target, true);

    // Update the container
    if let Ok(tree_node) = world.get::<TreeNode<(DebugIgnore,)>>(**node) {
        if let Some(container_entity) = tree_node.get_children_container() {
            commands.set_current_entity(container_entity);
            // Recurse with all the children of this node
            match target {
                UiNodeTarget::Entity(entity) => {
                    // Recuse over all components
                    let entity_location = world.get_entity_location(*entity).unwrap();
                    let component_types =
                        Vec::from(world.archetypes[entity_location.archetype as usize].types());
                    for &component_type in component_types.iter() {
                        update_node(
                            commands,
                            world,
                            resources,
                            ui_state,
                            UiNodeTarget::Component {
                                entity,
                                type_id: component_type.id(),
                                type_name: component_type.type_name(),
                            },
                            None,
                            None,
                        )
                    }
                }
                UiNodeTarget::Component {
                    entity, type_id, ..
                } => {
                    // Recurse over all fields
                    let type_registry = resources.get::<TypeRegistryArc>().unwrap();
                    let type_registry_read = type_registry.read();
                    if let Some(registration) = type_registry_read.get(type_id) {
                        if let Some(reflect_component) = registration.data::<ReflectComponent>() {
                            let reflect_component = unsafe {
                                let entity_location = world.get_entity_location(*entity).unwrap();
                                let entity_archetype =
                                    &world.archetypes[entity_location.archetype as usize];
                                reflect_component
                                    .reflect_component(&entity_archetype, entity_location.index)
                            };
                            drop(type_registry_read);
                            drop(type_registry);

                            dispatch_field(
                                commands,
                                world,
                                resources,
                                ui_state,
                                entity,
                                type_id,
                                reflect_component,
                            );
                        }
                    }
                }
                UiNodeTarget::Field {
                    entity,
                    component_type_id,
                    ..
                } => {
                    // Recurse over all fields
                    dispatch_field(
                        commands,
                        world,
                        resources,
                        ui_state,
                        entity,
                        component_type_id,
                        reflect_field.unwrap(),
                    );
                }
                UiNodeTarget::EditBox { .. } => {
                    unreachable!();
                }
            }
        }
    }
    commands.set_current_entity(parent_container);
}

fn dispatch_field(
    commands: &mut Commands,
    world: &World,
    resources: &mut Resources,
    ui_state: &mut UiState,
    entity: DebuggedEntity,
    component_type_id: TypeId,
    field: &dyn Reflect,
) {
    match field.reflect_ref() {
        bevy::reflect::ReflectRef::Struct(s) => {
            for (i, field) in s.iter_fields().enumerate() {
                let name = s.name_at(i).unwrap().to_string();
                update_node(
                    commands,
                    world,
                    resources,
                    ui_state,
                    UiNodeTarget::Field {
                        entity,
                        type_id: field.type_id(),
                        field_id: FieldId::new(field),
                        component_type_id,
                    },
                    Some(field),
                    Some(format!("{}: ", name)),
                );
            }
        }
        bevy::reflect::ReflectRef::TupleStruct(ts) => {
            for field in ts.iter_fields() {
                update_node(
                    commands,
                    world,
                    resources,
                    ui_state,
                    UiNodeTarget::Field {
                        entity,
                        type_id: field.type_id(),
                        field_id: FieldId::new(field),
                        component_type_id,
                    },
                    Some(field),
                    None,
                );
            }
        }
        bevy::reflect::ReflectRef::List(l) => {
            for (i, field) in l.iter().enumerate() {
                update_node(
                    commands,
                    world,
                    resources,
                    ui_state,
                    UiNodeTarget::Field {
                        entity,
                        type_id: field.type_id(),
                        field_id: FieldId::new(field),
                        component_type_id,
                    },
                    Some(field),
                    Some(format!("{}: ", i)),
                );
            }
        }
        bevy::reflect::ReflectRef::Map(m) => {
            for (_key, field) in m.iter() {
                update_node(
                    commands,
                    world,
                    resources,
                    ui_state,
                    UiNodeTarget::Field {
                        entity,
                        type_id: field.type_id(),
                        field_id: FieldId::new(field),
                        component_type_id,
                    },
                    Some(field),
                    Some("TODO: ".to_string()),
                );
            }
        }
        bevy::reflect::ReflectRef::Value(v) => {
            update_node_edit_box(
                commands,
                world,
                resources,
                ui_state,
                UiNodeTarget::EditBox {
                    entity,
                    // type_id: field.type_id(),
                    parent_field_id: FieldId::new(field),
                    component_type_id,
                },
                Some(v),
            );
        }
    }
}

fn update_node_edit_box(
    commands: &mut Commands,
    world: &World,
    resources: &mut Resources,
    ui_state: &mut UiState,
    target: UiNodeTarget,
    reflect_field: Option<&dyn Reflect>,
) {
    if let UiNodeTarget::EditBox {
        entity,
        component_type_id,
        ..
    } = target
    {
        let parent_container = commands.current_entity().unwrap();
        // Insert a new node if needed
        let edit_box_entity = *ui_state.map_ui_nodes.entry(target).or_insert_with(|| {
            trace!("inserting new node: {:?}", target);

            // Add a new edit box
            let ui_style = resources.get::<UiStyle>().unwrap();
            let mut root_entity = None;
            commands.with_children(|parent| {
                parent
                    .spawn(TextBundle {
                        style: Style {
                            align_self: AlignSelf::FlexStart,
                            align_content: AlignContent::Center,
                            margin: Rect::all(Val::Px(5.0)),
                            ..Default::default()
                        },
                        text: Text {
                            value: String::new(),
                            font: ui_style.font.clone(),
                            style: TextStyle {
                                font_size: 16.0,
                                color: ui_style.color_entity_text,
                                ..Default::default()
                            },
                        },
                        ..Default::default()
                    })
                    .with(edit_box::EditBox::default())
                    .with(Interaction::default())
                    .with(DebugIgnore)
                    .with(EditedField(None))
                    .with(SerializedField(None));
                root_entity = parent.current_entity();
            });

            UiEntity(root_entity.unwrap())
        });

        // Mark this node as alive
        ui_state.map_ui_nodes_is_alive.insert(target, true);

        if world.get::<edit_box::EditBox>(*edit_box_entity).is_ok() {
            let field = reflect_field.unwrap();
            let type_registry = resources.get::<TypeRegistry>().unwrap();
            let type_registry_read = type_registry.read();
            // Deserialize the EditedField component into the target field
            if let Ok(component) = world.get::<EditedField>(*edit_box_entity) {
                if let Some(edited_text) = &component.0 {
                    trace!("detected an edited field: {}", edited_text);
                    let mut deserializer = ron::de::Deserializer::from_str(edited_text).unwrap();
                    let reflect_deserializer = ReflectDeserializer::new(&type_registry_read);
                    // FIXME: fails if an inner type of the component is serializable but not registered?
                    match reflect_deserializer.deserialize(&mut deserializer) {
                        Ok(value) => {
                            // SECURITY: the only aliasing here is between the &dyn Reflect and the &World, which we own
                            // in this thread-local system. But this is still probably a very bad idea.
                            let field = unsafe {
                                (field as *const dyn Reflect as *mut dyn Reflect)
                                    .as_mut()
                                    .unwrap()
                            };
                            field.set(value).unwrap();
                            // We need to mark the component as mutated
                            ui_state.updated_components.push((
                                entity,
                                component_type_id,
                                edit_box_entity,
                            ));
                        }
                        Err(e) => {
                            warn!("Could not deserialize field: {}", e);
                            // FIXME: only clear the EditedField component
                            ui_state.updated_components.push((
                                entity,
                                component_type_id,
                                edit_box_entity,
                            ));
                        }
                    }
                }
            }
            // Update the SerializedField component
            let serializer = ReflectSerializer::new(field, &*type_registry_read);
            let serialized_field =
                ron::ser::to_string(&serializer).unwrap_or("Serialization failed".to_string());
            commands.insert_one(*edit_box_entity, SerializedField(Some(serialized_field)));
        }
        commands.set_current_entity(parent_container);
    } else {
        panic!(
            "update_node_edit_box called with a wrong target: {:?}",
            target
        );
    }
}

pub fn input(
    ev_scroll: Res<Events<MouseWheel>>,
    mut scroll: Local<EventReader<MouseWheel>>,
    mut ui_state: ResMut<UiState>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    for ev in scroll.iter(&ev_scroll) {
        // TODO: check if mouse is over the menu
        ui_state.scrolling_position += ev.y * 40.0;
    }
    if keyboard_input.just_pressed(KeyCode::F10) {
        ui_state.show = !ui_state.show;
        ui_state.show_progress = 1.0;
    }
}

pub fn unfocused_edit_box_event(
    event: Res<Events<edit_box::UnfocusedEvent>>,
    mut event_reader: Local<EventReader<edit_box::UnfocusedEvent>>,
    mut query: Query<&mut EditedField, With<edit_box::EditBox>>,
    mut query_ser: Query<
        (&mut Text, &mut SerializedField, &edit_box::EditBox),
        Changed<SerializedField>,
    >,
) {
    for event in event_reader.iter(&event) {
        trace!("Received event: {:?}", event);
        let mut edited_field = query.get_mut(event.entity).unwrap();
        let (_, serialized_text, _) = query_ser.get_mut(event.entity).unwrap();
        let serialized_text = serialized_text.0.clone().unwrap();
        let edited_text = event.text.as_str();
        let wrapped_text = wrap_serialized_value(serialized_text, edited_text);
        *edited_field = EditedField(Some(wrapped_text));
    }
    // Update the edit box from the serialized field if it's not being edited
    // TODO: encapsulate in edit_box module
    for (mut text, mut ser, edit_box) in query_ser.iter_mut() {
        if !edit_box.is_editing() {
            if let Some(new_text) = ser.0.take() {
                text.value
                    .replace_range(.., extract_serialized_value(new_text.as_str()));
            }
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

pub fn setup_ui_camera(commands: &mut Commands) {
    commands.spawn(CameraUiBundle::default());
}