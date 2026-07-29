// The prototype carries scaffolding that nothing calls yet: GameManager, EnemyFactory,
// FightFactory, the Status enum, and most Player fields. It is kept rather than deleted
// because it records the intended design, and the rewrite will decide what survives.
// Until then it would trip clippy's -D warnings in CI. Remove this once the rewrite lands.
#![allow(dead_code)]

mod components;
mod model;

use color_eyre::Result;
use dotenv::dotenv;
use ratatui::{DefaultTerminal, Frame, init as ratatui_init};
use tokio::{
    runtime::Runtime,
    sync::broadcast::{Sender, channel},
};

use components::{App, Component};

use model::{InputEvent, get_input_event};

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
            let mut app: App = App::new(
                event_reciever,
                event_sender.clone(),
                game_event_receiver,
                game_event_sender,
            );
            let event_handler_token = rt.spawn(event_handler(event_sender));
            let result = run(&mut terminal, &mut app).await;
            event_handler_token.abort();
            rt.shutdown_background();
            ratatui::restore();
            result
        }
        Err(_) => Ok(()),
    }
}

async fn event_handler(event_sender: Sender<InputEvent>) {
    loop {
        if let Some(event) = get_input_event() {
            let _ = event_sender.send(event);
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
            break Ok(());
        }
    }
}
