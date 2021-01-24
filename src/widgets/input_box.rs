use bevy::{
    prelude::*,
    text::DefaultTextPipeline,
    ui::{self, FocusPolicy},
};

// TODO
// - implement more control characters
// - test WASM for issues with out-of-order events
// - draw the cursor separately instead of inserting it into the text
// - text selection
// - clipboard

#[derive(Debug, Clone, Copy)]
pub struct Widget {
    pub text: Entity,
}

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub color_background: Handle<ColorMaterial>,
    pub font: Handle<Font>,
}

pub struct Builder {
    pub widget: Entity,
    pub text: Entity,
}

pub trait BuildInputBox {
    fn spawn_input_box(&mut self, style: Style, build_fn: Option<fn(&mut ChildBuilder)>)
        -> Builder;
}

impl BuildInputBox for Commands {
    fn spawn_input_box(
        &mut self,
        style: Style,
        build_fn: Option<fn(&mut ChildBuilder)>,
    ) -> Builder {
        let mut widget = None;
        let mut text = None;
        self.with_children(|parent| {
            widget = parent
                .spawn(NodeBundle {
                    style: ui::Style {
                        margin: Rect::all(Val::Px(2.0)),
                        ..Default::default()
                    },
                    material: style.color_background.clone(),
                    ..Default::default()
                })
                .with(Interaction::default())
                .current_entity();
            if let Some(build_fn) = build_fn.as_ref() {
                build_fn(parent);
            }

            parent
                .with_children(|parent| {
                    text = parent
                        .spawn(TextBundle {
                            style: ui::Style {
                                align_self: AlignSelf::FlexStart,
                                margin: Rect::all(Val::Px(2.0)),
                                size: Size {
                                    width: Val::Undefined,
                                    height: Val::Px(16.),
                                },
                                flex_shrink: 0.,
                                ..Default::default()
                            },
                            text: Text::with_section(
                                String::new(),
                                TextStyle {
                                    font: style.font.clone(),
                                    font_size: 16.0,
                                    color: Color::BLACK,
                                },
                                Default::default(),
                            ),
                            ..Default::default()
                        })
                        .with(FocusPolicy::Pass)
                        .with(Cursor::default())
                        .current_entity();
                    if let Some(build_fn) = build_fn.as_ref() {
                        build_fn(parent);
                    }
                })
                .with(Widget {
                    text: text.unwrap(),
                });
        });
        Builder {
            widget: widget.unwrap(),
            text: text.unwrap(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Cursor {
    cursor_pos: Option<usize>,
    cursor_char: char,
}

impl Cursor {
    fn focus(&mut self, text: &mut String, cursor: usize) {
        if self.cursor_pos.is_some() {
            warn!("This InputBox is already focused");
            return;
        }
        text.insert(cursor, self.cursor_char);
        self.cursor_pos = Some(cursor);
    }
    fn unfocus(&mut self, text: &mut String) {
        if let Some(cursor) = self.cursor_pos.take() {
            text.remove(cursor);
        } else {
            warn!("This InputBox is already unfocused");
        }
    }
    fn set_pos(&mut self, text: &mut String, mut new_cursor: usize) {
        if new_cursor > self.cursor_pos.unwrap() {
            new_cursor -= self.cursor_char.len_utf8();
        }
        self.unfocus(text);
        self.focus(text, new_cursor)
    }
    fn move_cursor_left(&mut self, text: &mut String) {
        if let Some(cursor) = &mut self.cursor_pos {
            if *cursor == 0 {
                return;
            }
            let mut start_previous_character = *cursor;
            loop {
                start_previous_character -= 1;
                if text.is_char_boundary(start_previous_character) {
                    break;
                }
            }
            let previous_char = text[start_previous_character..*cursor].to_string();
            let new_text = format!("{}{}", self.cursor_char, previous_char);
            text.replace_range(
                start_previous_character..*cursor + self.cursor_char.len_utf8(),
                new_text.as_str(),
            );
            *cursor = start_previous_character;
        }
    }
    fn move_cursor_right(&mut self, text: &mut String) {
        if let Some(cursor) = &mut self.cursor_pos {
            let cursor_size = self.cursor_char.len_utf8();
            if *cursor + cursor_size >= text.len() {
                return;
            }
            let mut end_next_character = *cursor + cursor_size;
            loop {
                end_next_character += 1;
                if text.is_char_boundary(end_next_character) {
                    break;
                }
            }
            let next_char = text[*cursor + cursor_size..end_next_character].to_string();
            let new_text = format!("{}{}", next_char, self.cursor_char);
            text.replace_range(*cursor..end_next_character, new_text.as_str());
            *cursor = end_next_character - cursor_size;
        }
    }

    fn insert_character(&mut self, text: &mut String, character: char) {
        if let Some(cursor) = &mut self.cursor_pos {
            text.insert(*cursor, character);
            *cursor += character.len_utf8();
        }
    }
    fn remove_character_before(&mut self, text: &mut String) {
        if let Some(cursor) = &mut self.cursor_pos {
            if *cursor == 0 {
                return;
            }
            let mut start_previous_character = *cursor;
            loop {
                start_previous_character -= 1;
                if text.is_char_boundary(start_previous_character) {
                    break;
                }
            }
            text.remove(start_previous_character);
            *cursor = start_previous_character;
        }
    }
    fn remove_character_after(&mut self, text: &mut String) {
        if let Some(cursor) = &mut self.cursor_pos {
            let cursor_size = self.cursor_char.len_utf8();
            if *cursor + cursor_size >= text.len() {
                return;
            }
            text.remove(*cursor + cursor_size);
        }
    }
    pub fn is_focused(&self) -> bool {
        self.cursor_pos.is_some()
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            cursor_pos: None,
            cursor_char: '𝄀',
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnfocusedEvent {
    pub entity: Entity,
    pub text: String,
    // Whether the unfocus was done with the Escape key
    pub canceled: bool,
}

#[derive(Debug)]
pub struct FocusedEvent {
    pub entity: Entity,
}

pub fn interact_mouse_system(
    query_widget: Query<(Entity, &Widget, &Interaction), Mutated<Interaction>>,
    mut query_cursor: Query<(&mut Cursor, &mut Text, &GlobalTransform)>,
    mut focused_events: ResMut<Events<FocusedEvent>>,
    text_pipeline: Res<DefaultTextPipeline>,
    windows: Res<Windows>,
) {
    if let Some(mouse_pos) = windows
        .get_primary()
        .and_then(|window| window.cursor_position())
    {
        for (entity, widget, interaction) in query_widget.iter() {
            let (mut cursor, mut text, transform) = query_cursor.get_mut(widget.text).unwrap();
            match *interaction {
                Interaction::Clicked => {
                    let mut new_cursor = text.sections[0].value.len();
                    if let Some(layout_info) = text_pipeline.get_glyphs(&widget.text) {
                        let layout_size =
                            Vec2::new(layout_info.size.width, layout_info.size.height);
                        let layout_origin = Vec2::new(
                            transform.translation[0] - layout_size[0] / 2.0,
                            transform.translation[1] - layout_size[1] / 2.0,
                        );
                        // FIXME: can we assume glyphs are in order? How does it work with left-to-right scripts?
                        for glyph in &layout_info.glyphs {
                            if mouse_pos.x < layout_origin.x + glyph.position.x {
                                new_cursor = glyph.byte_index;
                                break;
                            }
                        }
                    }
                    if let Some(_cursor) = cursor.cursor_pos {
                        // Move cursor under pointer
                        cursor.set_pos(&mut text.sections[0].value, new_cursor);
                    } else {
                        // Add cursor under pointer
                        cursor.focus(&mut text.sections[0].value, new_cursor);
                        focused_events.send(FocusedEvent { entity })
                    }
                }
                Interaction::Hovered => {}
                Interaction::None => {}
            }
        }
    }
}

pub fn interact_keyboard_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut character_events: EventReader<ReceivedCharacter>,
    mut query: Query<(&Parent, &mut Text, &mut Cursor)>,
    windows: Res<Windows>,
    mut unfocused_events: ResMut<Events<UnfocusedEvent>>,
) {
    for (parent, mut text, mut cursor) in query.iter_mut() {
        if cursor.cursor_pos.is_some() {
            let value = &mut text.sections[0].value;
            if keyboard_input.just_pressed(KeyCode::Return) {
                cursor.unfocus(value);
                unfocused_events.send(UnfocusedEvent {
                    entity: parent.0,
                    text: value.clone(),
                    canceled: false,
                });
                continue;
            } else if keyboard_input.just_pressed(KeyCode::Escape) {
                cursor.unfocus(value);
                unfocused_events.send(UnfocusedEvent {
                    entity: parent.0,
                    text: value.clone(),
                    canceled: true,
                });
            } else if keyboard_input.just_pressed(KeyCode::Left) {
                cursor.move_cursor_left(value);
            } else if keyboard_input.just_pressed(KeyCode::Right) {
                cursor.move_cursor_right(value);
            } else if keyboard_input.just_pressed(KeyCode::Back) {
                cursor.remove_character_before(value);
            } else if keyboard_input.just_pressed(KeyCode::Delete) {
                cursor.remove_character_after(value);
            }
            for character in character_events.iter() {
                if character.id == windows.get_primary().unwrap().id()
                    && !character.char.is_control()
                {
                    cursor.insert_character(value, character.char);
                }
            }
        }
    }
}
