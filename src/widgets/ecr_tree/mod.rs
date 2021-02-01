pub mod ecr;
pub mod leaf;
pub mod node;

use std::any::TypeId;

use bevy::{
    prelude::{ChildBuilder, Color, ColorMaterial, Commands, Entity, Handle, Reflect},
    reflect::TypeRegistry,
    text::Font,
    utils::HashMap,
};

use super::{check_box, input_box, tree_node};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Key {
    #[cfg(feature = "extra")]
    Resource {
        type_id: TypeId,
    },
    Entity {
        entity: Entity,
    },
    Component {
        entity: Entity,
        type_id: TypeId,
    },
    ReflectNode {
        address: usize,
        type_id: TypeId,
        // Used to differenciate enum variants
        variant_index: Option<usize>,
    },
    ReflectLeaf {
        address: usize,
        type_id: TypeId,
        // Used to differenciate enum variants
        variant_index: Option<usize>,
    },
}

#[derive(Debug)]
struct Entry {
    widget: Entity,
    inner: EntryType,
}

#[derive(Debug)]
enum EntryType {
    Node {
        // label: Entity,
        // expand_icon: Entity,
        container: Option<Entity>,
    },
    Leaf {
        // Copy of the current value
        value: Box<dyn Reflect>,
        // Whether [value] has changed this frame
        field_mutated: bool,
        // New value from the widget that needs to be applied
        widget_mutated: Option<Box<dyn Reflect>>,
    },
}

pub type FnSpawnWidget =
    fn(Key, &mut Commands, &mut State, TypeRegistry, &mut dyn Reflect, String, Entity) -> Entity;

#[derive(Default)]
pub struct State {
    /// Entity that has this component, should always be Some() but State needs to impl Default to be taken out of World
    state_entity: Option<Entity>,
    root_keys: Vec<Key>,
    entries: HashMap<Key, Entry>,
    entries_alive: HashMap<Key, bool>,
    specialized_widgets: HashMap<TypeId, FnSpawnWidget>,
    style: Style,
}

impl State {
    pub fn new(state_entity: Entity, style: Style) -> Self {
        let mut specialized_widgets = HashMap::default();
        specialized_widgets.insert(
            TypeId::of::<bool>(),
            leaf::spawn_widget_bool as FnSpawnWidget,
        );
        Self {
            state_entity: Some(state_entity),
            entries: Default::default(),
            entries_alive: Default::default(),
            specialized_widgets,
            style,
            root_keys: Default::default(),
        }
    }
    pub fn get_root_keys_mut(&mut self) -> &mut Vec<Key> {
        &mut self.root_keys
    }
    pub fn get_widgets_mut(&mut self) -> &mut HashMap<TypeId, FnSpawnWidget> {
        &mut self.specialized_widgets
    }
}

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub font: Handle<Font>,
    pub style_node: tree_node::Style,
    pub color_node_text: Color,
    pub color_background: Handle<ColorMaterial>,
    pub style_input_box: input_box::Style,
    pub icon_chevron_down: Handle<ColorMaterial>,
    pub icon_chevron_up: Handle<ColorMaterial>,
    pub style_check_box: check_box::Style,
}

pub struct EntryAccess {
    state_entity: Entity,
    key: Key,
}

pub fn with_debug_ignore(parent: &mut ChildBuilder) {
    parent.with(crate::DebugIgnore);
}

fn serialize_reflect(reflect: &dyn Reflect) -> Option<String> {
    let serializable = reflect.serializable()?;
    let serialize = serializable.borrow();
    ron::to_string(&serialize).ok()
}
