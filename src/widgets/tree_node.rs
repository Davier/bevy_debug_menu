use bevy::{
    prelude::{
        trace, BuildChildren, ButtonBundle, ChildBuilder, ColorMaterial, Commands,
        DespawnRecursiveExt, Entity, Events, FlexDirection, Handle, Interaction, Mutated,
        NodeBundle, Parent, Query, ResMut, Visible, With,
    },
    ui,
};

pub struct Widget {
    pub style: Style,
    children_container: Option<Entity>,
    build_fn: Option<fn(&mut ChildBuilder)>,
}

#[derive(Debug)]
pub struct Button {
    widget: Entity,
}

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub color_root_container: Handle<ColorMaterial>,
    pub color_button: Handle<ColorMaterial>,
    pub color_button_hovered: Option<Handle<ColorMaterial>>,
    pub color_button_clicked: Option<Handle<ColorMaterial>>,
    pub color_button_expanded: Option<Handle<ColorMaterial>>,
    pub color_children_container: Handle<ColorMaterial>,
    pub node_style_button: ui::Style,
    pub node_style_children_container: ui::Style,
}

pub struct Builder {
    pub widget: Entity,
    pub button: Entity,
}

pub trait BuildTreeNode {
    fn spawn_tree_node(&mut self, style: Style, build_fn: Option<fn(&mut ChildBuilder)>)
        -> Builder;
}

impl BuildTreeNode for Commands {
    fn spawn_tree_node(
        &mut self,
        style: Style,
        build_fn: Option<fn(&mut ChildBuilder)>,
    ) -> Builder {
        let mut widget = None;
        let mut button = None;
        let color_button = style.color_button.clone();

        self.with_children(|parent| {
            // Root container
            parent.spawn(NodeBundle {
                style: ui::Style {
                    flex_direction: FlexDirection::ColumnReverse,
                    flex_shrink: 0.,
                    // z_index: ZIndex::Auto,
                    ..Default::default()
                },
                material: style.color_root_container.clone(),
                visible: Visible {
                    is_transparent: true,
                    ..Default::default()
                },
                ..Default::default()
            });
            widget = parent.current_entity();
            if let Some(build_fn) = build_fn.as_ref() {
                build_fn(parent);
            }
            parent.with_children(|parent| {
                parent
                    // Button that expands the node
                    .spawn(ButtonBundle {
                        style: style.node_style_button.clone(),
                        material: color_button,
                        ..Default::default()
                    })
                    .with(Button {
                        widget: widget.unwrap(),
                    });
                if let Some(build_fn) = build_fn.as_ref() {
                    build_fn(parent);
                }
                button = parent.current_entity();
            });

            parent.with(Widget {
                children_container: None,
                style,
                build_fn,
            });
        });

        Builder {
            widget: widget.unwrap(),
            button: button.unwrap(),
        }
    }
}

impl Widget {
    /// Expand or retract the node, creating or destroying the children container
    pub fn toggle_expand(&mut self, tree_node_entity: Entity, commands: &mut Commands) {
        if let Some(container_entity) = self.children_container.take() {
            trace!("removing child container");
            commands.despawn_recursive(container_entity);
        } else {
            trace!("inserting child container");
            commands.set_current_entity(tree_node_entity);
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
                if let Some(build_fn) = self.build_fn.as_ref() {
                    build_fn(parent);
                }
                self.children_container = Some(parent.current_entity().unwrap());
            });
        }
    }

    pub fn is_expanded(&self) -> bool {
        self.children_container.is_some()
    }

    pub fn get_children_container(&self) -> Option<Entity> {
        self.children_container
    }
}

#[derive(Debug)]
pub struct ExpandedEvent {
    pub widget: Entity,
    pub expanded: bool,
}

// Update the button material and expand/retract the children when clicked
pub fn interact_button_system(
    mut interaction_query: Query<
        (&Interaction, &mut Handle<ColorMaterial>, &Parent),
        (Mutated<Interaction>, With<Button>),
    >,
    mut tree_node_query: Query<&mut Widget>,
    commands: &mut Commands,
    mut expanded_events: ResMut<Events<ExpandedEvent>>,
) {
    for (interaction, mut material, Parent(parent)) in interaction_query.iter_mut() {
        let mut tree_node = tree_node_query.get_mut(*parent).unwrap();
        match *interaction {
            Interaction::Clicked => {
                if let Some(color_button_clicked) = &tree_node.style.color_button_clicked {
                    *material = color_button_clicked.clone();
                }
                trace!("Node toggled");
                tree_node.toggle_expand(*parent, commands);
                expanded_events.send(ExpandedEvent {
                    widget: *parent,
                    expanded: tree_node.is_expanded(),
                });
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

impl std::fmt::Debug for Widget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TreeNode {{ ")?;
        self.children_container.fmt(f)?;
        self.style.fmt(f)?;
        write!(f, " }}")
    }
}
