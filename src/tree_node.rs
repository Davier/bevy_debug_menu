use bevy::{ecs::{DynamicBundle}, prelude::*, ui::FocusPolicy};

// FIXME: is there a way to avoid being generic over the marker component?
//        right now we need to store it in order to spawn the children container
pub struct TreeNode<T: MarkerComponents = ()> {
    container_entity: Option<Entity>,
    components: Option<T>,
    pub style: TreeNodeStyle,
}

pub trait MarkerComponents: DynamicBundle + Send + Sync + Clone + 'static {}
impl<T: DynamicBundle + Send + Sync + Clone + 'static> MarkerComponents for T {} 

#[derive(Clone)]
pub struct TreeNodeStyle {
    pub color_root_container: Handle<ColorMaterial>,
    pub color_button: Handle<ColorMaterial>,
    pub color_button_hovered: Option<Handle<ColorMaterial>>,
    pub color_button_clicked: Option<Handle<ColorMaterial>>,
    pub color_button_expanded: Option<Handle<ColorMaterial>>,
    pub color_children_container: Handle<ColorMaterial>,
    pub node_style_button: Style,
    pub node_style_children_container: Style,
}

pub struct TreeNodeButton;

pub trait BuildTreeNode<T: MarkerComponents> {
    // Spawn a tree node as a child of the current entity and returns its root entity.
    // After the call the current entity is set to the node button, so that it's easy to add some text or image inside
    // TODO: how to return &mut self to allow chain commands but still give back to the root entity?
    fn spawn_tree_node(&mut self, style: TreeNodeStyle, components: Option<T>) -> Entity;
}

impl<T: MarkerComponents> BuildTreeNode<T> for Commands {
    fn spawn_tree_node(&mut self, style: TreeNodeStyle, components: Option<T>) -> Entity {
        let mut root_container = None;
        let mut button = None;
        let color_button = style.color_button.clone();

        self.with_children(|parent| {
            // Root container
            parent.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::ColumnReverse,
                    ..Default::default()
                },
                material: style.color_root_container.clone(),
                visible: Visible {
                    is_transparent: true,
                    ..Default::default()
                },
                ..Default::default()
            });
            if let Some(components) = components.clone() {
                parent.with_bundle(components);
            }
            parent.with_children(|parent| {
                parent
                    // Button that expands the node
                    .spawn(ButtonBundle {
                        style: style.node_style_button.clone(),
                        material: color_button,
                        focus_policy: FocusPolicy::Block,
                        ..Default::default()
                    })
                    .with(TreeNodeButton);
                if let Some(components) = components.as_ref() {
                    parent.with_bundle(components.clone());
                }
                button = parent.current_entity();
            });
            root_container = parent.current_entity();

            parent.with(TreeNode {
                container_entity: None,
                components: components,
                style,
            });
        });

        self.set_current_entity(button.unwrap());
        root_container.unwrap()
    }
}

impl<T: MarkerComponents> TreeNode<T> {
    // Expand or retract the node, creating or destroying the children container
    pub fn toggle_expand(&mut self, entity: Entity, commands: &mut Commands) {
        if let Some(container_entity) = self.container_entity.take() {
            trace!("removing child container");
            commands.despawn_recursive(container_entity);
        } else {
            trace!("inserting child container");
            commands.set_current_entity(entity);
            commands.with_children(|parent| {
                parent
                    // Children container
                    .spawn(NodeBundle {
                        style: self.style.node_style_children_container.clone(),
                        material: self.style.color_children_container.clone(),
                        visible: Visible {
                            is_transparent: true,
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                if let Some(components) = self.components.clone() {
                    parent.with_bundle(components);
                }
                self.container_entity = Some(parent.current_entity().unwrap());
            });
        }
    }

    pub fn is_expanded(&self) -> bool {
        self.container_entity.is_some()
    }

    pub fn get_children_container(&self) -> Option<Entity> {
        self.container_entity
    }
}

// Update the button material and expand/retract the children when clicked
pub fn interact_button<T: MarkerComponents + 'static>(
    mut interaction_query: Query<
        (&Interaction, &mut Handle<ColorMaterial>, &Parent),
        (Mutated<Interaction>, With<TreeNodeButton>),
    >,
    mut tree_node_query: Query<&mut TreeNode<T>>,
    commands: &mut Commands,
) {
    for (interaction, mut material, Parent(parent)) in interaction_query.iter_mut() {
        let mut tree_node = tree_node_query.get_mut(*parent).unwrap();
        match *interaction {
            Interaction::Clicked => {
                if let Some(color_button_clicked) = &tree_node.style.color_button_clicked {
                    *material = color_button_clicked.clone();
                }
                tree_node.toggle_expand(*parent, commands);
            }
            Interaction::Hovered => {
                if let Some(color_button_hovered) = &tree_node.style.color_button_hovered {
                    *material = color_button_hovered.clone();
                }
            }
            Interaction::None => {
                if tree_node.is_expanded() && tree_node.style.color_button_expanded.is_some() {
                    *material = tree_node.style.color_button_expanded.clone().unwrap();
                } else {
                    *material = tree_node.style.color_button.clone();
                }
            }
        }
    }
}
