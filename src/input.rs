//! Key mapping.
//!
//! This layer is screen-agnostic: it turns a keypress into a generic intent
//! and lets [`crate::app::App`] decide what that means for the focused zone.
//! Keeping it dumb is what makes "arrows and Enter drive everything" hold
//! across every screen without special cases here.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use crate::action::Action;
use crate::app::Screen;

pub fn map(key: KeyEvent, screen: Screen, awaiting_dismiss: bool) -> Option<Action> {
    // Only act on presses; ignore releases and repeats.
    if key.kind != KeyEventKind::Press {
        return None;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Some(Action::Quit),
        KeyCode::Char('r') if screen == Screen::GameOver => return Some(Action::Restart),
        _ => {}
    }

    // While a result is on screen, only dismissing it is possible -- so the
    // player cannot blunder past an outcome they have not read.
    if awaiting_dismiss {
        return match key.code {
            KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Confirm),
            _ => None,
        };
    }

    match key.code {
        KeyCode::Left | KeyCode::Char('h') => Some(Action::NavLeft),
        KeyCode::Right | KeyCode::Char('l') => Some(Action::NavRight),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::NavUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::NavDown),
        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Confirm),

        KeyCode::Backspace => Some(Action::Undo),
        KeyCode::Delete | KeyCode::Char('c') => Some(Action::Clear),
        KeyCode::Tab => Some(Action::EndTurn),
        KeyCode::Char(c @ '1'..='5') => Some(Action::AddComponent(c as usize - '1' as usize)),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, ratatui::crossterm::event::KeyModifiers::NONE)
    }

    #[test]
    fn quit_works_from_every_screen() {
        for screen in [Screen::Map, Screen::Combat, Screen::Event, Screen::GameOver] {
            assert_eq!(
                map(press(KeyCode::Char('q')), screen, false),
                Some(Action::Quit)
            );
        }
    }

    #[test]
    fn arrows_and_enter_produce_intents_on_every_screen() {
        // The whole point: no screen needs its own key vocabulary.
        for screen in [Screen::Map, Screen::Combat, Screen::Event] {
            assert_eq!(
                map(press(KeyCode::Left), screen, false),
                Some(Action::NavLeft)
            );
            assert_eq!(
                map(press(KeyCode::Right), screen, false),
                Some(Action::NavRight)
            );
            assert_eq!(map(press(KeyCode::Up), screen, false), Some(Action::NavUp));
            assert_eq!(
                map(press(KeyCode::Down), screen, false),
                Some(Action::NavDown)
            );
            assert_eq!(
                map(press(KeyCode::Enter), screen, false),
                Some(Action::Confirm)
            );
        }
    }

    #[test]
    fn number_keys_remain_a_shortcut_for_spell_slots() {
        assert_eq!(
            map(press(KeyCode::Char('1')), Screen::Combat, false),
            Some(Action::AddComponent(0))
        );
        assert_eq!(
            map(press(KeyCode::Char('5')), Screen::Combat, false),
            Some(Action::AddComponent(4))
        );
        assert_eq!(map(press(KeyCode::Char('6')), Screen::Combat, false), None);
    }

    #[test]
    fn key_releases_are_ignored() {
        let mut key = press(KeyCode::Enter);
        key.kind = KeyEventKind::Release;
        assert_eq!(map(key, Screen::Combat, false), None);
    }

    #[test]
    fn a_pending_result_swallows_everything_except_dismiss_and_quit() {
        assert_eq!(map(press(KeyCode::Left), Screen::Event, true), None);
        assert_eq!(
            map(press(KeyCode::Enter), Screen::Event, true),
            Some(Action::Confirm)
        );
        assert_eq!(
            map(press(KeyCode::Char('q')), Screen::Event, true),
            Some(Action::Quit)
        );
    }

    #[test]
    fn restart_is_only_offered_once_the_run_is_over() {
        assert_eq!(
            map(press(KeyCode::Char('r')), Screen::GameOver, false),
            Some(Action::Restart)
        );
        assert_ne!(
            map(press(KeyCode::Char('r')), Screen::Combat, false),
            Some(Action::Restart)
        );
    }
}
