use bevy::{
    diagnostic::{DiagnosticId, Diagnostics},
    prelude::*,
    ui,
};

use crate::DebugIgnore;

#[derive(Debug)]
pub struct DiagnosticList {
    style: Style,
}

#[derive(Debug)]
pub struct DiagnosticListItem {
    id: DiagnosticId,
}

#[derive(Debug, Clone)]
pub struct Style {
    pub font: Handle<Font>,
    pub font_size: f32,
    pub color_background: Handle<ColorMaterial>,
    pub color_box: Handle<ColorMaterial>,
    pub style_box: ui::Style,
}

pub fn spawn(commands: &mut Commands, style: &Style) -> Entity {
    let mut entity = None;
    commands.with_children(|parent| {
        parent
            .spawn(NodeBundle {
                style: ui::Style {
                    flex_direction: FlexDirection::ColumnReverse,
                    position: Rect {
                        left: Val::Undefined,
                        top: Val::Px(0.0), // We use this for vertical scrolling
                        bottom: Val::Undefined,
                        right: Val::Undefined,
                    },
                    size: Size {
                        width: Val::Percent(100.),
                        height: Val::Undefined, // Height will grow as needed
                    },
                    flex_shrink: 0.,
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
            .with(DiagnosticList {
                style: style.clone(),
            })
            .with(DebugIgnore);
        entity = Some(parent.current_entity().unwrap());
    });
    entity.unwrap()
}

pub fn update_system(
    commands: &mut Commands,
    query_list: Query<(Entity, &Children, &DiagnosticList)>,
    mut query_item: Query<(&DiagnosticListItem, &mut Text)>,
    diagnostics: Res<Diagnostics>,
) {
    for (entity, items, widget) in query_list.iter() {
        // If # of diagnostics changed, rebuild the list
        if diagnostics.iter().count() != query_item.iter_mut().count() {
            trace!("Rebuilding diagnostics list");
            for item in items.iter() {
                commands.despawn_recursive(*item);
            }
            for diagnostic in diagnostics.iter() {
                commands.set_current_entity(entity);
                commands.with_children(|parent| {
                    parent
                        .spawn(NodeBundle {
                            style: widget.style.style_box.clone(),
                            material: widget.style.color_box.clone(),
                            ..Default::default()
                        })
                        .with(DebugIgnore)
                        .with_children(|parent| {
                            parent
                                .spawn(TextBundle {
                                    text: Text::with_section(
                                        format!(
                                            "{}:",
                                            &diagnostic.name
                                                [diagnostic.name.len().saturating_sub(40)..]
                                        ),
                                        TextStyle {
                                            font: widget.style.font.clone(),
                                            font_size: widget.style.font_size,
                                            color: Color::WHITE,
                                        },
                                        TextAlignment {
                                            vertical: VerticalAlign::Center,
                                            horizontal: HorizontalAlign::Left,
                                        },
                                    ),
                                    style: ui::Style {
                                        align_self: AlignSelf::FlexStart,
                                        size: Size {
                                            width: Val::Undefined,
                                            height: Val::Px(widget.style.font_size),
                                        },
                                        flex_shrink: 0.,
                                        flex_grow: 1.,
                                        margin: Rect::all(Val::Px(4.0)),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .with(DebugIgnore)
                                .spawn(TextBundle {
                                    text: Text::with_section(
                                        String::new(),
                                        TextStyle {
                                            font: widget.style.font.clone(),
                                            font_size: widget.style.font_size,
                                            color: Color::WHITE,
                                        },
                                        TextAlignment {
                                            vertical: VerticalAlign::Center,
                                            horizontal: HorizontalAlign::Right,
                                        },
                                    ),
                                    style: ui::Style {
                                        align_self: AlignSelf::FlexEnd,
                                        size: Size {
                                            width: Val::Undefined,
                                            height: Val::Px(widget.style.font_size),
                                        },
                                        flex_shrink: 0.,
                                        margin: Rect::all(Val::Px(4.0)),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .with(DiagnosticListItem { id: diagnostic.id })
                                .with(DebugIgnore);
                        });
                });
            }
        }

        // For each text, update
        for (item, mut text) in query_item.iter_mut() {
            if let Some(diagnostic) = diagnostics.get(item.id) {
                text.sections[0].value = format!("{:.3}", diagnostic.average().unwrap_or(f64::NAN));
            }
        }
    }
}
