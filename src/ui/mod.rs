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

use crate::app::{App, Screen};

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
