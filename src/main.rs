mod model;
mod components;

use env_logger;
use dotenv::dotenv;
use color_eyre::Result;
use ratatui::{init as ratatui_init, DefaultTerminal, Frame};
use tokio::{
    sync::broadcast::{channel, Sender, Receiver},
    runtime::Runtime
};

use components::{
    App,
    Component,
    Components
};

use model::{get_input_event, InputEvent};

use crate::model::GameEvent;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    dotenv().ok();
    match Runtime::new() {
        Ok(rt) => {
            let mut terminal: DefaultTerminal = ratatui_init();
            let (event_sender, event_reciever) = channel::<InputEvent>(10);
            let (game_event_sender, game_event_receiver) = channel::<GameEvent>(10);
            let mut app: App = App::new(event_reciever, event_sender.clone(), game_event_receiver, game_event_sender);
            let event_handler_token = rt.spawn(event_handler(event_sender));
            let result = run(&mut terminal, &mut app).await;
            event_handler_token.abort();
            rt.shutdown_background();
            ratatui::restore();
            result
        },
        Err(_) => Ok(()),
    }
}

async fn event_handler(event_sender: Sender<InputEvent>) {
    loop {
        match get_input_event() {
            Some(event) => {
                let _ = event_sender.send(event);
            },
            None => {}
        }
    }
}

async fn run(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        app.handle_events().await;
        app.update();
        terminal.draw(|frame: &mut Frame| {
            app.render(frame, frame.area());
        })?;
        if app.should_close {
            break Ok(())
        }
    }
}

