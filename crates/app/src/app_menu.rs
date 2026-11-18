use egui::{Event, Key, Modifiers};

pub const SETTINGS_ID: &str = "app-menu.settings";
pub const QUIT_ID: &str = "app-menu.quit";
pub const CLOSE_ID: &str = "app-menu.close";
pub const UNDO_ID: &str = "app-menu.undo";
pub const REDO_ID: &str = "app-menu.redo";
pub const CUT_ID: &str = "app-menu.cut";
pub const COPY_ID: &str = "app-menu.copy";
pub const PASTE_ID: &str = "app-menu.paste";
pub const SELECT_ALL_ID: &str = "app-menu.select-all";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditCommand {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppMenuAction {
    Settings,
    Quit,
    CloseWindow,
    Edit(EditCommand),
}

pub fn action_for(id: &str) -> Option<AppMenuAction> {
    Some(match id {
        SETTINGS_ID => AppMenuAction::Settings,
        QUIT_ID => AppMenuAction::Quit,
        CLOSE_ID => AppMenuAction::CloseWindow,
        UNDO_ID => AppMenuAction::Edit(EditCommand::Undo),
        REDO_ID => AppMenuAction::Edit(EditCommand::Redo),
        CUT_ID => AppMenuAction::Edit(EditCommand::Cut),
        COPY_ID => AppMenuAction::Edit(EditCommand::Copy),
        PASTE_ID => AppMenuAction::Edit(EditCommand::Paste),
        SELECT_ALL_ID => AppMenuAction::Edit(EditCommand::SelectAll),
        _ => return None,
    })
}

impl EditCommand {
    pub fn needs_clipboard(self) -> bool {
        self == Self::Paste
    }

    pub fn events(self, clipboard: Option<&str>) -> Vec<Event> {
        match self {
            Self::Cut => vec![Event::Cut],
            Self::Copy => vec![Event::Copy],
            Self::Paste => clipboard
                .map(|text| {
                    text.replace("\r\n", " ")
                        .replace('\r', " ")
                        .replace('\n', " ")
                })
                .filter(|text| !text.is_empty())
                .map(Event::Paste)
                .into_iter()
                .collect(),
            Self::SelectAll => key_stroke(Key::A, Modifiers::COMMAND),
            Self::Undo => key_stroke(Key::Z, Modifiers::COMMAND),
            Self::Redo => key_stroke(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT),
        }
    }
}

fn key_stroke(key: Key, modifiers: Modifiers) -> Vec<Event> {
    [true, false]
        .into_iter()
        .map(|pressed| Event::Key {
            key,
            physical_key: Some(key),
            pressed,
            repeat: false,
            modifiers,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_menu_id_maps_to_its_action() {
        let expected = [
            (SETTINGS_ID, AppMenuAction::Settings),
            (QUIT_ID, AppMenuAction::Quit),
            (CLOSE_ID, AppMenuAction::CloseWindow),
            (UNDO_ID, AppMenuAction::Edit(EditCommand::Undo)),
            (REDO_ID, AppMenuAction::Edit(EditCommand::Redo)),
            (CUT_ID, AppMenuAction::Edit(EditCommand::Cut)),
            (COPY_ID, AppMenuAction::Edit(EditCommand::Copy)),
            (PASTE_ID, AppMenuAction::Edit(EditCommand::Paste)),
            (SELECT_ALL_ID, AppMenuAction::Edit(EditCommand::SelectAll)),
        ];
        for (id, action) in expected {
            assert_eq!(action_for(id), Some(action), "{id}");
        }
    }

    #[test]
    fn tray_and_unknown_ids_are_not_app_menu_actions() {
        assert_eq!(action_for("1234"), None);
        assert_eq!(action_for(""), None);
    }

    #[test]
    fn only_paste_needs_the_clipboard() {
        assert!(EditCommand::Paste.needs_clipboard());
        assert!(!EditCommand::Copy.needs_clipboard());
        assert!(!EditCommand::SelectAll.needs_clipboard());
    }

    #[test]
    fn cut_and_copy_become_the_matching_egui_events() {
        assert_eq!(EditCommand::Cut.events(None), vec![Event::Cut]);
        assert_eq!(EditCommand::Copy.events(None), vec![Event::Copy]);
    }

    #[test]
    fn paste_carries_the_clipboard_text_including_thai() {
        assert_eq!(
            EditCommand::Paste.events(Some("สวัสดี\r\nครับ\rผม\nชื่อ")),
            vec![Event::Paste("สวัสดี ครับ ผม ชื่อ".to_owned())]
        );
    }

    #[test]
    fn paste_with_an_empty_or_missing_clipboard_does_nothing() {
        assert!(EditCommand::Paste.events(None).is_empty());
        assert!(EditCommand::Paste.events(Some("")).is_empty());
    }

    fn pressed_key(events: &[Event]) -> Option<(Key, Modifiers)> {
        events.iter().find_map(|event| match event {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => Some((*key, *modifiers)),
            _ => None,
        })
    }

    #[test]
    fn select_all_undo_and_redo_are_command_key_strokes() {
        assert_eq!(
            pressed_key(&EditCommand::SelectAll.events(None)),
            Some((Key::A, Modifiers::COMMAND))
        );
        assert_eq!(
            pressed_key(&EditCommand::Undo.events(None)),
            Some((Key::Z, Modifiers::COMMAND))
        );
        assert_eq!(
            pressed_key(&EditCommand::Redo.events(None)),
            Some((Key::Z, Modifiers::COMMAND | Modifiers::SHIFT))
        );
    }

    #[test]
    fn a_key_stroke_presses_then_releases() {
        let events = EditCommand::Undo.events(None);
        let pressed: Vec<bool> = events
            .iter()
            .filter_map(|event| match event {
                Event::Key { pressed, .. } => Some(*pressed),
                _ => None,
            })
            .collect();
        assert_eq!(pressed, [true, false]);
    }
}
