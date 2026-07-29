//! The browser shell.
//!
//! Everything below the `App` boundary is shared verbatim with the terminal
//! build. All this does is translate the browser's key events into [`Key`] and
//! hand ratzilla a closure to draw with -- which is only possible because
//! `game/` and `ui/` never knew what a terminal was.

use std::cell::RefCell;
use std::rc::Rc;

use ratzilla::DomBackend;
use ratzilla::WebRenderer;
use ratzilla::event::{KeyCode, KeyEvent};
use ratzilla::ratatui::Terminal;
use ratzilla::web_sys::console;

use tui_game::action::Action;
use tui_game::app::App;
use tui_game::game::content::Content;
use tui_game::input::Key;
use tui_game::{input, ui};

/// Translate a browser keypress into the platform-neutral [`Key`].
fn to_key(event: &KeyEvent) -> Option<Key> {
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

pub fn run() {
    // Panics land in the browser console with a readable message instead of
    // the default "unreachable executed".
    std::panic::set_hook(Box::new(|info| {
        console::error_1(&format!("panic: {info}").into());
    }));

    let content = match Content::load() {
        Ok(content) => content,
        Err(error) => {
            console::error_1(&format!("failed to load content: {error}").into());
            return;
        }
    };

    let app = Rc::new(RefCell::new(App::from_content(content)));

    let backend = match DomBackend::new_by_id("terminal") {
        Ok(backend) => backend,
        Err(error) => {
            console::error_1(&format!("failed to create backend: {error}").into());
            return;
        }
    };
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(error) => {
            console::error_1(&format!("failed to create terminal: {error}").into());
            return;
        }
    };

    let input_app = Rc::clone(&app);
    let handler = terminal.on_key_event(move |event| {
        let Some(key) = to_key(&event) else { return };
        let mut app = input_app.borrow_mut();
        let (screen, awaiting) = (app.screen, app.awaiting_dismiss());
        let Some(action) = input::map(key, screen, awaiting) else {
            return;
        };
        // There is nothing to quit to in a browser tab, so q and Esc are
        // simply inert here rather than freezing the page on a dead loop.
        if action == Action::Quit {
            return;
        }
        app.apply(action);
    });
    if let Err(error) = handler {
        console::error_1(&format!("failed to attach key handler: {error}").into());
        return;
    }

    terminal.draw_web(move |frame| {
        let mut app = app.borrow_mut();
        app.tick();
        ui::render(frame, &app);
    });
}
