//! Terminal setup and teardown, and the loop.

use std::time::Duration;

use color_eyre::eyre::Result;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event};

use tui_game::app::App;
use tui_game::game::content::Content;
use tui_game::{input, ui};

fn main() -> Result<()> {
    color_eyre::install()?;

    // Load content before touching the terminal, so a content error prints as
    // an ordinary message instead of into a half-initialised alternate screen.
    let content = Content::load()?;

    let mut terminal = ratatui::init();
    install_panic_hook();

    let result = run(&mut terminal, App::from_content(content)?);

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

fn run(terminal: &mut DefaultTerminal, mut app: App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, &app))?;

        // Polling is what stops this loop spinning a core, and it doubles as
        // the timer that paces the combat log.
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && let Some(action) = input::map(key, app.screen, app.awaiting_dismiss())
        {
            app.apply(action);
        }

        app.tick();
    }
    Ok(())
}
