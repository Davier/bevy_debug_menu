use bevy::prelude::*;

pub struct EditBox {
    cursor_pos: Option<usize>,
    cursor_char: char,
}

impl Default for EditBox {
    fn default() -> Self {
        Self {
            cursor_pos: None,
            cursor_char: '|',
        }
    }
}

impl EditBox {
    fn focus(&mut self, text: &mut String) {
        if self.cursor_pos.is_some() {
            warn!("This EditBox is already focused");
            return;
        }
        let cursor = text.len();
        text.push(self.cursor_char);
        self.cursor_pos = Some(cursor);
    }
    fn unfocus(&mut self, text: &mut String) {
        if let Some(cursor) = self.cursor_pos.take() {
            text.remove(cursor);
        } else {
            warn!("This EditBox is already unfocused");
        }
    }
    fn move_cursor_left(&mut self, text: &mut String) {
        if let Some(cursor) = &mut self.cursor_pos {
            if *cursor <= 0 {
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
            let new_text = format!("|{}", previous_char);
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
            let new_text = format!("{}|", next_char);
            text.replace_range(*cursor..end_next_character, new_text.as_str());
            *cursor = end_next_character - cursor_size;
        }
    }

    fn insert_character(&mut self, text: &mut String, character: char) {
        if let Some(cursor) = &mut self.cursor_pos {
            text.insert(*cursor, character);
            *cursor = *cursor + character.len_utf8();
        }
    }
    fn remove_character_before(&mut self, text: &mut String) {
        if let Some(cursor) = &mut self.cursor_pos {
            if *cursor <= 0 {
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
            *cursor = *cursor;
        }
    }
    pub fn is_editing(&self) -> bool {
        self.cursor_pos.is_some()
    }
}

#[derive(Clone, Debug)]
pub struct UnfocusedEvent {
    pub entity: Entity,
    pub text: String,
    // Whether the focus was done with the Escape key
    pub canceled: bool,
}

pub struct FocusedEvent {
    pub entity: Entity,
}

pub fn interact_mouse(
    mut query: Query<(Entity, &mut Text, &mut EditBox, &Interaction), Mutated<Interaction>>,
    mut focused_events: ResMut<Events<FocusedEvent>>,
) {
    for (entity, mut text, mut edit_box, interaction) in query.iter_mut() {
        match *interaction {
            Interaction::Clicked => {
                if let Some(_cursor) = edit_box.cursor_pos {
                    // TODO: Move cursor under pointer
                } else {
                    // Add cursor under pointer
                    edit_box.focus(&mut text.value);
                    focused_events.send(FocusedEvent { entity })
                }
            }
            Interaction::Hovered => {}
            Interaction::None => {}
        }
    }
    // TODO: clicking anywhere else should unfocus
}

pub fn interact_keyboard(
    keyboard_input: Res<Input<KeyCode>>,
    event_char: Res<Events<ReceivedCharacter>>,
    mut event_reader_char: Local<EventReader<ReceivedCharacter>>,
    mut query: Query<(Entity, &mut Text, &mut EditBox)>,
    windows: Res<Windows>,
    mut unfocused_events: ResMut<Events<UnfocusedEvent>>,
) {
    for (entity, mut text, mut edit_box) in query.iter_mut() {
        if edit_box.cursor_pos.is_some() {
            if keyboard_input.just_pressed(KeyCode::Return) {
                edit_box.unfocus(&mut text.value);
                unfocused_events.send(UnfocusedEvent {
                    entity,
                    text: text.value.clone(),
                    canceled: true,
                });
                continue;
            } else if keyboard_input.just_pressed(KeyCode::Escape) {
                edit_box.unfocus(&mut text.value);
                unfocused_events.send(UnfocusedEvent {
                    entity,
                    text: text.value.clone(),
                    canceled: false,
                });
            } else if keyboard_input.just_pressed(KeyCode::Left) {
                edit_box.move_cursor_left(&mut text.value);
            } else if keyboard_input.just_pressed(KeyCode::Right) {
                edit_box.move_cursor_right(&mut text.value);
            } else if keyboard_input.just_pressed(KeyCode::Back) {
                edit_box.remove_character_before(&mut text.value);
            } else if keyboard_input.just_pressed(KeyCode::Delete) {
                edit_box.remove_character_after(&mut text.value);
            }
            for character in event_reader_char.iter(&event_char) {
                if character.id == windows.get_primary().unwrap().id()
                    && !character.char.is_control()
                {
                    edit_box.insert_character(&mut text.value, character.char);
                }
            }
        }
    }
}
