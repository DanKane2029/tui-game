use dotenv::dotenv;
use color_eyre::Result;
use crossterm::event::{self, Event};
use ratatui::{DefaultTerminal, Frame};
use game::{
    enemies::{get_enemies, Enemy},
    spell::{get_spells, Spell}
};

mod cli;
mod game;

fn main() -> Result<()> {
    // dotenv().ok();
    // env_logger::init();
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> Result<()> {
    loop {
        log::debug!("loop");
        terminal.draw(render)?;
        if matches!(event::read()?, Event::Key(_)) {
            break Ok(());
        }
    }
}

fn render(frame: &mut Frame) {
    frame.render_widget("hello world", frame.area());
}
