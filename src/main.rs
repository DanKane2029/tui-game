mod model;
mod components;

use env_logger;
use dotenv::dotenv;
use color_eyre::Result;
use ratatui::{init as ratatui_init, DefaultTerminal, Frame};
use tokio::runtime::Runtime;

use components::{
    App,
    Component
};

use model::{get_input_event, InputEvent};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    dotenv().ok();
    let tokio_runtime = Runtime::new();
    let mut terminal: DefaultTerminal = ratatui_init();
    let ecent_handler = tokio::spawn(event_handler());
    let mut app = App::new();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

async fn event_handler() {
    loop {
        log::debug!("in event")
    }
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        app.handle_key_events();
        app.update();
        terminal.draw(|frame: &mut Frame| {
            app.render(frame, frame.area());
        })?;
        match get_input_event() {
            Some(event) => {
                match event {
                    InputEvent::Esc => break Ok(()),
                    _ => ()
                }
            },
            None => (),
        }
        
    }
}

