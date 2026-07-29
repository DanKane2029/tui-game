//! Rendering. Every function here takes `&App` -- nothing in this module can
//! mutate the simulation.

pub mod combat;
pub mod event;
pub mod game_over;
pub mod map;
pub mod options;
pub mod replacement;
pub mod reward;
pub mod shop;
pub mod title;
pub mod widgets;

use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::{App, Screen};

/// The largest area the game will draw into.
///
/// A terminal, or a maximised browser window, can be far bigger than the game
/// needs. Rather than let panels stretch to absurd proportions, everything is
/// drawn into a centred area of at most this size.
pub const MAX_WIDTH: u16 = 110;
pub const MAX_HEIGHT: u16 = 34;

/// The centred region every screen draws into. Use this instead of
/// `frame.area()`.
pub fn stage(frame: &Frame) -> Rect {
    let area = frame.area();
    let width = area.width.min(MAX_WIDTH);
    let height = area.height.min(MAX_HEIGHT);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

pub fn render(f: &mut Frame, app: &App) {
    // A pending replacement is modal: it can be raised from more than one
    // screen, and nothing else should be interactable until it resolves.
    if app.pending.is_some() {
        replacement::render(f, app);
        return;
    }

    match app.screen {
        Screen::Title => title::render(f, app),
        Screen::Options => options::render(f, app),
        Screen::Map => map::render(f, app),
        Screen::Combat => combat::render(f, app),
        Screen::Reward => reward::render(f, app),
        Screen::Shop => shop::render(f, app),
        Screen::Event => event::render(f, app),
        Screen::GameOver => game_over::render(f, app),
    }
}
