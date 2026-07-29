//! Key mapping.
//!
//! Deliberately platform-neutral. The terminal build and the browser build
//! deliver keypresses through completely different APIs, so both translate
//! into [`Key`] and share every line of the mapping below. Nothing here knows
//! what a terminal is.
//!
//! It is also screen-agnostic: it turns a keypress into a generic intent and
//! lets [`crate::app::App`] decide what that means for the focused zone.

use crate::action::Action;
use crate::app::Screen;

/// A keypress, independent of where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Left,
    Right,
    Up,
    Down,
    Enter,
    Backspace,
    Delete,
    Tab,
    Esc,
}

pub fn map(key: Key, screen: Screen, awaiting_dismiss: bool) -> Option<Action> {
    match key {
        Key::Char('q') | Key::Esc => return Some(Action::Quit),
        Key::Char('r') if screen == Screen::GameOver => return Some(Action::Restart),
        _ => {}
    }

    // While a result is on screen, only dismissing it is possible -- so the
    // player cannot blunder past an outcome they have not read.
    if awaiting_dismiss {
        return match key {
            Key::Enter | Key::Char(' ') => Some(Action::Confirm),
            _ => None,
        };
    }

    match key {
        Key::Left | Key::Char('h') => Some(Action::NavLeft),
        Key::Right | Key::Char('l') => Some(Action::NavRight),
        Key::Up | Key::Char('k') => Some(Action::NavUp),
        Key::Down | Key::Char('j') => Some(Action::NavDown),
        Key::Enter | Key::Char(' ') => Some(Action::Confirm),

        Key::Backspace => Some(Action::Undo),
        Key::Delete | Key::Char('c') => Some(Action::Clear),
        Key::Tab => Some(Action::EndTurn),
        Key::Char(c @ '1'..='5') => Some(Action::AddComponent(c as usize - '1' as usize)),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_works_from_every_screen() {
        for screen in [Screen::Map, Screen::Combat, Screen::Event, Screen::GameOver] {
            assert_eq!(map(Key::Char('q'), screen, false), Some(Action::Quit));
        }
    }

    #[test]
    fn arrows_and_enter_produce_intents_on_every_screen() {
        // The whole point: no screen needs its own key vocabulary.
        for screen in [Screen::Map, Screen::Combat, Screen::Event] {
            assert_eq!(map(Key::Left, screen, false), Some(Action::NavLeft));
            assert_eq!(map(Key::Right, screen, false), Some(Action::NavRight));
            assert_eq!(map(Key::Up, screen, false), Some(Action::NavUp));
            assert_eq!(map(Key::Down, screen, false), Some(Action::NavDown));
            assert_eq!(map(Key::Enter, screen, false), Some(Action::Confirm));
        }
    }

    #[test]
    fn number_keys_remain_a_shortcut_for_spell_slots() {
        assert_eq!(
            map(Key::Char('1'), Screen::Combat, false),
            Some(Action::AddComponent(0))
        );
        assert_eq!(
            map(Key::Char('5'), Screen::Combat, false),
            Some(Action::AddComponent(4))
        );
        assert_eq!(map(Key::Char('6'), Screen::Combat, false), None);
    }

    #[test]
    fn a_pending_result_swallows_everything_except_dismiss_and_quit() {
        assert_eq!(map(Key::Left, Screen::Event, true), None);
        assert_eq!(map(Key::Enter, Screen::Event, true), Some(Action::Confirm));
        assert_eq!(map(Key::Char('q'), Screen::Event, true), Some(Action::Quit));
    }

    #[test]
    fn restart_is_only_offered_once_the_run_is_over() {
        assert_eq!(
            map(Key::Char('r'), Screen::GameOver, false),
            Some(Action::Restart)
        );
        assert_ne!(
            map(Key::Char('r'), Screen::Combat, false),
            Some(Action::Restart)
        );
    }
}
