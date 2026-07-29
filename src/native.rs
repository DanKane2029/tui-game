//! The terminal shell: setup, teardown, and the blocking event loop.

use std::time::Duration;

use color_eyre::eyre::Result;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event};

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use tui_game::app::App;
use tui_game::game::content::Content;
use tui_game::input::Key;
use tui_game::{input, ui};

/// Translate a crossterm keypress into the platform-neutral [`Key`] that
/// `input` understands. The browser build does the same from its own event
/// type, and the two then share every line of the mapping.
fn to_key(event: KeyEvent) -> Option<Key> {
    // Only act on presses; ignore releases and repeats.
    if event.kind != KeyEventKind::Press {
        return None;
    }
    Some(match event.code {
        KeyCode::Char(c) => Key::Char(c),
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Enter => Key::Enter,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Tab => Key::Tab,
        KeyCode::Esc => Key::Esc,
        _ => return None,
    })
}

pub fn run() -> Result<()> {
    color_eyre::install()?;

    // Load content before touching the terminal, so a content error prints as
    // an ordinary message instead of into a half-initialised alternate screen.
    let content = Content::load()?;

    let mut terminal = ratatui::init();
    install_panic_hook();

    let result = game_loop(&mut terminal, App::from_content(content));

    ratatui::restore();
    result
}

/// Restore the terminal *before* the panic message prints. Without this a
/// panic leaves the terminal in raw mode on the alternate screen and wrecks
/// the user's shell.
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        previous(info);
    }));
}

fn game_loop(terminal: &mut DefaultTerminal, mut app: App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &app))?;

        // Polling is what stops this loop spinning a core, and it doubles as
        // the timer that paces the combat log.
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(event) = event::read()?
            && let Some(key) = to_key(event)
            && let Some(action) = input::map(key, app.screen, app.awaiting_dismiss())
        {
            app.apply(action);
        }

        app.tick();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    #[test]
    fn key_releases_are_ignored() {
        // Terminals report press and release; acting on both would double
        // every keystroke. The web shell has no equivalent, which is why this
        // filtering lives here rather than in `input`.
        let mut event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(to_key(event), Some(Key::Enter));

        event.kind = KeyEventKind::Release;
        assert_eq!(to_key(event), None);
    }

    #[test]
    fn unmapped_keys_are_dropped() {
        assert_eq!(
            to_key(KeyEvent::new(KeyCode::F(5), KeyModifiers::NONE)),
            None
        );
    }
}
