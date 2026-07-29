//! Rendering. Every function here takes `&App` -- nothing in this module can
//! mutate the simulation.

pub mod combat;
pub mod event;
pub mod game_over;
pub mod map;
pub mod widgets;

use ratatui::Frame;

use crate::app::{App, Screen};

pub fn render(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::Map => map::render(f, app),
        Screen::Combat => combat::render(f, app),
        Screen::Event => event::render(f, app),
        Screen::GameOver => game_over::render(f, app),
    }
}
