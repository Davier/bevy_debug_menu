use bevy::{
    prelude::{
        BuildChildren, ButtonBundle, ChildBuilder, ColorMaterial, Entity, Events, Handle,
        Interaction, Mutated, NodeBundle, Query, ResMut,
    },
    ui,
};

#[derive(Debug)]
pub struct Widget {
    buttons: Vec<Entity>,
    selection: Option<usize>,
    style: Style,
}

#[derive(Debug)]
pub struct Button {
    widget: Entity,
}

#[derive(Debug, Clone)]
pub struct Style {
    pub style_container: ui::Style,
    pub color_container: Handle<ColorMaterial>,
    pub style_button: ui::Style,
    pub color_button: Handle<ColorMaterial>,
    pub color_button_hovered: Option<Handle<ColorMaterial>>,
    pub color_button_clicked: Option<Handle<ColorMaterial>>,
    pub color_button_selected: Option<Handle<ColorMaterial>>,
}

pub struct Builder {
    pub widget: Entity,
    pub buttons: Vec<Entity>,
}

pub trait BuildRadioButtons {
    fn spawn_radio_buttons(
        &mut self,
        number: usize,
        selection: Option<usize>,
        style: Style,
        build_fn: Option<fn(&mut ChildBuilder)>,
    ) -> Builder;
}

impl<'a> BuildRadioButtons for ChildBuilder<'a> {
    fn spawn_radio_buttons(
        &mut self,
        number: usize,
        selection: Option<usize>,
        style: Style,
        build_fn: Option<fn(&mut ChildBuilder)>,
    ) -> Builder {
        let mut buttons = Vec::new();

        // Root container
        self.spawn(NodeBundle {
            style: style.style_container.clone(),
            material: style.color_container.clone(),
            ..Default::default()
        })
        .with(crate::DebugIgnore);
        let root = self.current_entity().unwrap();
        for i in 0..number {
            self.with_children(|parent| {
                // Buttons
                parent
                    .spawn(ButtonBundle {
                        style: style.style_button.clone(),
                        material: if Some(i) == selection && style.color_button_selected.is_some() {
                            style.color_button_selected.clone().unwrap()
                        } else {
                            style.color_button.clone()
                        },
                        ..Default::default()
                    })
                    .with(Button { widget: root });
                if let Some(build_fn) = build_fn.as_ref() {
                    build_fn(parent);
                }
                buttons.push(parent.current_entity().unwrap());
            });
        }
        self.with(Widget {
            buttons: buttons.clone(),
            selection,
            style,
        });
        Builder {
            widget: root,
            buttons,
        }
    }
}

impl Widget {
    pub fn select(&mut self, button: Entity) -> Option<(usize, Entity)> {
        let previous = self.selection;
        let new = self.buttons.iter().position(|&entity| entity == button);
        if Some(new) == Some(previous) {
            self.selection = None;
        } else {
            self.selection = new;
        }
        previous.map(|index| (index, self.buttons[index]))
    }

    pub fn is_selected(&self, button: Entity) -> bool {
        let position = self.buttons.iter().position(|&entity| entity == button);
        position == self.selection
    }
}

#[derive(Debug)]
pub struct SelectionChangedEvent {
    pub widget: Entity,
    pub new_selection: Option<usize>,
    pub previous_selection: Option<usize>,
}

// Update the button materials and ensure only one is selected
pub fn interact_system(
    interaction_query: Query<(Entity, &Interaction, &Button), Mutated<Interaction>>,
    mut material_query: Query<&mut Handle<ColorMaterial>>,
    mut container_query: Query<&mut Widget>,
    mut selection_changed_events: ResMut<Events<SelectionChangedEvent>>,
) {
    for (entity, interaction, radio_button) in interaction_query.iter() {
        let mut material = material_query.get_mut(entity).unwrap();
        let mut widget = container_query.get_mut(radio_button.widget).unwrap();
        match *interaction {
            Interaction::Clicked => {
                if let Some(color_button_clicked) = &widget.style.color_button_clicked {
                    *material = color_button_clicked.clone();
                }
                let previous = widget.select(entity);
                if let Some((_previous_index, previous_button)) = previous {
                    let mut previous_material = material_query.get_mut(previous_button).unwrap();
                    *previous_material = widget.style.color_button.clone();
                }
                selection_changed_events.send(SelectionChangedEvent {
                    widget: radio_button.widget,
                    new_selection: widget.selection,
                    previous_selection: previous.map(|(index, _entity)| index),
                });
            }
            Interaction::Hovered => {
                if let Some(color_button_hovered) = &widget.style.color_button_hovered {
                    *material = color_button_hovered.clone();
                }
            }
            Interaction::None => {
                if widget.is_selected(entity) && widget.style.color_button_selected.is_some() {
                    *material = widget.style.color_button_selected.clone().unwrap();
                } else {
                    *material = widget.style.color_button.clone();
                }
            }
        }
    }
}
