use bevy::{prelude::*, ui};

#[derive(Debug)]
pub struct Widget {
    pub style: Style,
    checked: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub icon_toggle_on: Handle<ColorMaterial>,
    pub icon_toggle_off: Handle<ColorMaterial>,
    pub icon_toggle_on_hovered: Option<Handle<ColorMaterial>>,
    pub icon_toggle_off_hovered: Option<Handle<ColorMaterial>>,
}

pub struct Builder {
    pub widget: Entity,
}

pub trait BuildCheckBox {
    fn spawn_check_box(
        &mut self,
        checked: bool,
        style: Style,
        build_fn: Option<fn(&mut ChildBuilder)>,
    ) -> Builder;
}

impl BuildCheckBox for Commands {
    fn spawn_check_box(
        &mut self,
        checked: bool,
        style: Style,
        build_fn: Option<fn(&mut ChildBuilder)>,
    ) -> Builder {
        let mut root = None;
        self.with_children(|parent| {
            root = parent
                .spawn(ImageBundle {
                    style: ui::Style {
                        align_self: AlignSelf::Center,
                        flex_shrink: 0.,
                        size: Size {
                            width: Val::Px(32.0),
                            height: Val::Undefined,
                        },
                        margin: Rect {
                            left: Val::Px(5.0),
                            right: Val::Px(5.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    material: if checked {
                        style.icon_toggle_on.clone()
                    } else {
                        style.icon_toggle_off.clone()
                    },
                    ..Default::default()
                })
                .with(Widget { checked, style })
                .with(Interaction::default())
                .current_entity();
            if let Some(build_fn) = build_fn.as_ref() {
                build_fn(parent);
            }
        });
        Builder {
            widget: root.unwrap(),
        }
    }
}

impl Widget {
    pub fn set(&mut self, checked: bool) {
        self.checked = checked;
    }
    pub fn is_checked(&self) -> bool {
        self.checked
    }
    pub fn toggle(&mut self) -> bool {
        self.checked = !self.checked;
        self.checked
    }
}

#[derive(Debug)]
pub struct ToggledEvent {
    pub entity: Entity,
    pub checked: bool,
}

pub fn update_mutated_system(
    mut query_mutated: Query<
        (&Widget, &mut Handle<ColorMaterial>, Option<&Interaction>),
        (With<Widget>, Or<(Mutated<Widget>, Mutated<Interaction>)>),
    >,
) {
    for (checkbox, mut material, interaction) in query_mutated.iter_mut() {
        if checkbox.is_checked() {
            if interaction == Some(&Interaction::Hovered)
                && checkbox.style.icon_toggle_on_hovered.is_some()
            {
                *material = checkbox.style.icon_toggle_on_hovered.clone().unwrap();
            } else {
                *material = checkbox.style.icon_toggle_on.clone();
            }
        } else if interaction == Some(&Interaction::Hovered)
            && checkbox.style.icon_toggle_off_hovered.is_some()
        {
            *material = checkbox.style.icon_toggle_off_hovered.clone().unwrap();
        } else {
            *material = checkbox.style.icon_toggle_off.clone();
        }
    }
}

pub fn interact_system(
    mut toggled_events: ResMut<Events<ToggledEvent>>,
    mut query_toggled: Query<(Entity, &mut Widget, &Interaction), Mutated<Interaction>>,
) {
    for (entity, mut checkbox, interaction) in query_toggled.iter_mut() {
        if *interaction == Interaction::Clicked {
            toggled_events.send(ToggledEvent {
                entity,
                checked: checkbox.toggle(),
            });
        }
    }
}
