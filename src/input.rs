//! Key mapping. Which action a key produces depends on the active screen, so
//! the same arrow keys can mean different things without any ambiguity.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use crate::action::Action;
use crate::app::Screen;

pub fn map(key: KeyEvent, screen: Screen, awaiting_dismiss: bool) -> Option<Action> {
    // Ignore key-release and repeat events; only act on a press.
    if key.kind != KeyEventKind::Press {
        return None;
    }

    // Global keys work everywhere.
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return Some(Action::Quit),
        KeyCode::Char('r') if screen == Screen::GameOver => return Some(Action::Restart),
        _ => {}
    }

    if awaiting_dismiss {
        return match key.code {
            KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Continue),
            _ => None,
        };
    }

    match screen {
        Screen::Map => match key.code {
            KeyCode::Left | KeyCode::Up | KeyCode::Char('h') => Some(Action::MapPrev),
            KeyCode::Right | KeyCode::Down | KeyCode::Char('l') => Some(Action::MapNext),
            KeyCode::Enter | KeyCode::Char(' ') => Some(Action::MapEnter),
            _ => None,
        },

        Screen::Combat => match key.code {
            KeyCode::Char(c @ '1'..='5') => Some(Action::AddComponent(c as usize - '1' as usize)),
            KeyCode::Backspace => Some(Action::Undo),
            KeyCode::Delete | KeyCode::Char('c') => Some(Action::Clear),
            KeyCode::Up | KeyCode::Left => Some(Action::TargetPrev),
            KeyCode::Down | KeyCode::Right => Some(Action::TargetNext),
            KeyCode::Enter => Some(Action::Cast),
            KeyCode::Tab | KeyCode::Char('e') => Some(Action::EndTurn),
            _ => None,
        },

        Screen::Event => match key.code {
            KeyCode::Up | KeyCode::Left => Some(Action::ChoicePrev),
            KeyCode::Down | KeyCode::Right => Some(Action::ChoiceNext),
            KeyCode::Enter | KeyCode::Char(' ') => Some(Action::ChoiceSelect),
            _ => None,
        },

        Screen::GameOver => None,
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
    fn number_keys_select_spell_slots_zero_indexed() {
        assert_eq!(
            map(press(KeyCode::Char('1')), Screen::Combat, false),
            Some(Action::AddComponent(0))
        );
        assert_eq!(
            map(press(KeyCode::Char('5')), Screen::Combat, false),
            Some(Action::AddComponent(4))
        );
        // There is no sixth slot.
        assert_eq!(map(press(KeyCode::Char('6')), Screen::Combat, false), None);
    }

    #[test]
    fn the_same_key_means_different_things_per_screen() {
        assert_eq!(
            map(press(KeyCode::Enter), Screen::Combat, false),
            Some(Action::Cast)
        );
        assert_eq!(
            map(press(KeyCode::Enter), Screen::Map, false),
            Some(Action::MapEnter)
        );
    }

    #[test]
    fn key_releases_are_ignored() {
        let mut key = press(KeyCode::Enter);
        key.kind = KeyEventKind::Release;
        assert_eq!(map(key, Screen::Combat, false), None);
    }

    #[test]
    fn a_pending_result_swallows_everything_except_dismiss_and_quit() {
        assert_eq!(
            map(press(KeyCode::Char('1')), Screen::Combat, true),
            None,
            "must not act on the fight while a result is on screen"
        );
        assert_eq!(
            map(press(KeyCode::Enter), Screen::Combat, true),
            Some(Action::Continue)
        );
        assert_eq!(
            map(press(KeyCode::Char('q')), Screen::Combat, true),
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
