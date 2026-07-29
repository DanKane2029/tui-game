use crate::components::{Component, FightDisplay, SpellSelect};
use crate::model::{Enemy, Fight, FightManager, GameEvent, InputEvent, Player, Spell, get_spells};

use ratatui::layout::{Constraint, Direction, Layout};
use tokio::sync::broadcast::{Receiver, Sender};

pub struct App {
    title: String,
    pub event_receiver: Receiver<InputEvent>,
    game_event_receiver: Receiver<GameEvent>,
    pub should_close: bool,

    player: Player,
    enemy_list: Vec<Enemy>,
    spell_list: Vec<Spell>,

    layout: Layout,
    fight_display: FightDisplay,
    spell_select: SpellSelect,
}

impl App {
    pub fn new(
        event_receiver: Receiver<InputEvent>,
        event_sender: Sender<InputEvent>,
        game_event_receiver: Receiver<GameEvent>,
        game_event_sender: Sender<GameEvent>,
    ) -> Self {
        let spell_list = get_spells();
        let enemy_list = vec![
            Enemy::new("TEST ENEMY 1".to_string(), 10),
            Enemy::new("TEST ENEMY 2".to_string(), 15),
            Enemy::new("TEST ENEMY 3".to_string(), 20),
        ];

        let mut app = Self {
            title: String::from("App Title"),
            event_receiver,
            game_event_receiver,
            should_close: false,

            player: Player::new(),
            enemy_list: enemy_list.clone(),
            spell_list,

            layout: Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(66), Constraint::Percentage(34)]),
            fight_display: FightDisplay::new(FightManager::new(
                Player::new(),
                Fight::new(enemy_list.clone()[0..3].to_vec()),
            )),
            spell_select: SpellSelect::new(
                [None, None, None, None, None],
                event_sender.subscribe(),
                game_event_sender,
            ),
        };
        app.init();
        app
    }
}

impl Component for App {
    fn init(&mut self) {
        let _ = self.spell_select.add_spell(self.spell_list[0].clone());
        let _ = self.spell_select.add_spell(self.spell_list[2].clone());
    }

    async fn handle_events(&mut self) {
        if let Ok(event) = self.event_receiver.try_recv()
            && let InputEvent::Esc = event
        {
            self.should_close = true;
        }

        self.spell_select.handle_events().await;

        if let Ok(_game_event) = self.game_event_receiver.try_recv() {}
    }

    fn update(&mut self) {
        self.spell_select.update();
    }

    fn render(&mut self, f: &mut ratatui::Frame<'_>, rect: ratatui::prelude::Rect) {
        let app_layout = self.layout.split(rect);
        self.fight_display.render(f, app_layout[0]);

        self.spell_select.render(f, app_layout[1]);
    }
}
