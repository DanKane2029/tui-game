//! The shell: owns all state, routes actions, and drives the loop.

use color_eyre::eyre::Result;
use rand::seq::IndexedRandom;

use crate::action::Action;
use crate::game::combat::{Combat, Command, Phase};
use crate::game::content::Content;
use crate::game::encounter;
use crate::game::event::{MapEvent, apply_outcome};
use crate::game::map::{NodeId, NodeKind};
use crate::game::run::Run;

/// A fixed set of screens, so an enum beats dynamic dispatch: the compiler
/// points at every match that needs updating when a variant is added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Map,
    Combat,
    Event,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcome {
    Won,
    Lost,
}

#[derive(Debug, Clone)]
pub struct ActiveEvent {
    pub event: MapEvent,
    /// Set once a choice has been made; the player dismisses it to continue.
    pub result: Option<String>,
}

/// View state. The simulation never sees any of this.
#[derive(Debug, Clone, Default)]
pub struct UiState {
    pub map_cursor: usize,
    pub choice_cursor: usize,
    /// How many combat log entries are currently on screen. Grows over time so
    /// an enemy turn reads as a sequence of beats rather than one instant dump.
    pub revealed: usize,
}

pub struct App {
    pub content: Content,
    pub run: Run,
    pub screen: Screen,
    pub combat: Option<Combat>,
    pub event: Option<ActiveEvent>,
    pub outcome: Option<RunOutcome>,
    pub ui: UiState,
    pub should_quit: bool,
}

impl App {
    pub fn new(content: Content, seed: u64) -> Self {
        let run = Run::new(&content, seed);
        Self {
            content,
            run,
            screen: Screen::Map,
            combat: None,
            event: None,
            outcome: None,
            ui: UiState::default(),
            should_quit: false,
        }
    }

    pub fn from_content(content: Content) -> Result<Self> {
        Ok(Self::new(content, rand::random()))
    }

    /// True while a result is on screen waiting to be dismissed. Input is
    /// suppressed so the player cannot act past it by accident.
    pub fn awaiting_dismiss(&self) -> bool {
        self.event.as_ref().is_some_and(|e| e.result.is_some())
    }

    /// Combat log entries currently revealed.
    pub fn visible_log(&self) -> &[crate::game::combat::Event] {
        match &self.combat {
            Some(c) => &c.log[..self.ui.revealed.min(c.log.len())],
            None => &[],
        }
    }

    pub fn nodes_available(&self) -> Vec<NodeId> {
        self.run.available()
    }

    /// Reveal pending log entries a beat at a time.
    pub fn tick(&mut self) {
        if let Some(combat) = &self.combat
            && self.ui.revealed < combat.log.len()
        {
            self.ui.revealed += 1;
        }
    }

    pub fn apply(&mut self, action: Action) {
        match action {
            Action::Quit => self.should_quit = true,
            Action::Restart => *self = App::new(self.content.clone(), rand::random()),

            Action::MapPrev => {
                let n = self.nodes_available().len().max(1);
                self.ui.map_cursor = (self.ui.map_cursor + n - 1) % n;
            }
            Action::MapNext => {
                let n = self.nodes_available().len().max(1);
                self.ui.map_cursor = (self.ui.map_cursor + 1) % n;
            }
            Action::MapEnter => self.enter_selected_node(),

            Action::ChoicePrev => {
                if let Some(active) = &self.event {
                    let n = active.event.choices.len().max(1);
                    self.ui.choice_cursor = (self.ui.choice_cursor + n - 1) % n;
                }
            }
            Action::ChoiceNext => {
                if let Some(active) = &self.event {
                    let n = active.event.choices.len().max(1);
                    self.ui.choice_cursor = (self.ui.choice_cursor + 1) % n;
                }
            }
            Action::ChoiceSelect => self.resolve_event_choice(),
            Action::Continue => self.dismiss_result(),

            Action::AddComponent(slot) => self.combat_command(Command::AddComponent(slot)),
            Action::Undo => self.combat_command(Command::UndoComponent),
            Action::Clear => self.combat_command(Command::ClearBuild),
            Action::TargetNext => self.combat_command(Command::TargetNext),
            Action::TargetPrev => self.combat_command(Command::TargetPrev),
            Action::Cast => self.combat_command(Command::Cast),
            Action::EndTurn => self.combat_command(Command::EndTurn),
        }
    }

    fn enter_selected_node(&mut self) {
        let available = self.nodes_available();
        let Some(&id) = available.get(self.ui.map_cursor) else {
            return;
        };
        self.run.enter(id);

        match self.run.map.node(id).kind {
            NodeKind::Fight | NodeKind::Boss => {
                let budget = self.run.encounter_budget();
                let enemies = encounter::generate(&self.content.enemies, budget, &mut self.run.rng);
                self.combat = Some(Combat::new(enemies));
                self.ui.revealed = 0;
                self.screen = Screen::Combat;
            }
            NodeKind::Event => {
                let picked = self.content.events.choose(&mut self.run.rng).cloned();
                match picked {
                    Some(event) => {
                        self.event = Some(ActiveEvent {
                            event,
                            result: None,
                        });
                        self.ui.choice_cursor = 0;
                        self.screen = Screen::Event;
                    }
                    // No events authored: treat the node as already cleared
                    // rather than stranding the player on an empty screen.
                    None => self.advance_after_node(),
                }
            }
        }
    }

    fn resolve_event_choice(&mut self) {
        let Some(active) = &self.event else { return };
        if active.result.is_some() {
            return;
        }
        let Some(choice) = active.event.choices.get(self.ui.choice_cursor).cloned() else {
            return;
        };

        let text = apply_outcome(
            &mut self.run.player,
            choice.outcome,
            &self.content.spells,
            &mut self.run.rng,
        );

        if let Some(active) = &mut self.event {
            active.result = Some(text);
        }
    }

    fn dismiss_result(&mut self) {
        self.event = None;
        if !self.run.player.is_alive() {
            self.finish(RunOutcome::Lost);
        } else {
            self.advance_after_node();
        }
    }

    fn combat_command(&mut self, cmd: Command) {
        let Some(combat) = self.combat.as_mut() else {
            return;
        };
        combat.apply(&mut self.run.player, &self.content.rules, cmd);

        if !self.run.player.is_alive() {
            combat.note_player_death();
            self.finish(RunOutcome::Lost);
            return;
        }

        if self.combat.as_ref().is_some_and(|c| c.phase == Phase::Won) {
            if self.run.is_complete() {
                self.finish(RunOutcome::Won);
            } else {
                self.combat = None;
                self.advance_after_node();
            }
        }
    }

    fn advance_after_node(&mut self) {
        self.combat = None;
        self.event = None;
        self.ui.map_cursor = 0;
        self.ui.revealed = 0;
        self.screen = Screen::Map;
    }

    fn finish(&mut self, outcome: RunOutcome) {
        self.outcome = Some(outcome);
        self.screen = Screen::GameOver;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        App::new(Content::load().expect("content parses"), 1)
    }

    #[test]
    fn a_new_app_starts_on_the_map_with_one_choice() {
        let app = app();
        assert_eq!(app.screen, Screen::Map);
        assert_eq!(app.nodes_available().len(), 1);
        assert!(app.outcome.is_none());
    }

    #[test]
    fn entering_the_first_node_starts_a_fight() {
        let mut app = app();
        app.apply(Action::MapEnter);
        assert_eq!(app.screen, Screen::Combat);
        let combat = app.combat.as_ref().expect("a fight was created");
        assert!(!combat.enemies.is_empty());
    }

    #[test]
    fn the_map_cursor_wraps_and_never_indexes_out_of_bounds() {
        let mut app = app();
        for _ in 0..20 {
            app.apply(Action::MapNext);
            assert!(app.ui.map_cursor < app.nodes_available().len().max(1));
        }
        for _ in 0..20 {
            app.apply(Action::MapPrev);
            assert!(app.ui.map_cursor < app.nodes_available().len().max(1));
        }
    }

    #[test]
    fn building_an_incantation_beyond_your_mana_is_refused() {
        let mut app = app();
        app.apply(Action::MapEnter);
        app.run.player.mana = 1;

        // Slot 0 is Ember at 1 mana: affordable once, not twice.
        app.apply(Action::AddComponent(0));
        app.apply(Action::AddComponent(0));

        let combat = app.combat.as_ref().unwrap();
        assert_eq!(combat.build.len(), 1, "second component should be refused");
    }

    #[test]
    fn casting_spends_mana_and_damages_the_target() {
        let mut app = app();
        app.apply(Action::MapEnter);

        let before_mana = app.run.player.mana;
        let target = app.combat.as_ref().unwrap().target;
        let before_hp = app.combat.as_ref().unwrap().enemies[target].hp;

        app.apply(Action::AddComponent(0));
        app.apply(Action::Cast);

        assert!(app.run.player.mana < before_mana, "mana was not spent");
        let combat = app.combat.as_ref().unwrap();
        assert!(
            combat.enemies[target].hp < before_hp || !combat.enemies[target].is_alive(),
            "target took no damage"
        );
        assert!(combat.build.is_empty(), "build should clear after casting");
    }

    #[test]
    fn dying_ends_the_run() {
        let mut app = app();
        app.apply(Action::MapEnter);
        app.run.player.hp = 1;
        // End turn so the enemies get to swing.
        for _ in 0..10 {
            if app.screen == Screen::GameOver {
                break;
            }
            app.apply(Action::EndTurn);
        }
        assert_eq!(app.screen, Screen::GameOver);
        assert_eq!(app.outcome, Some(RunOutcome::Lost));
    }

    #[test]
    fn clearing_every_enemy_returns_to_the_map() {
        let mut app = app();
        app.apply(Action::MapEnter);
        for enemy in &mut app.combat.as_mut().unwrap().enemies {
            enemy.hp = 1;
        }
        app.run.player.mana = 99;
        for _ in 0..12 {
            if app.screen != Screen::Combat {
                break;
            }
            app.apply(Action::AddComponent(0));
            app.apply(Action::Cast);
            app.apply(Action::TargetNext);
        }
        assert_eq!(app.screen, Screen::Map, "fight did not resolve");
    }

    #[test]
    fn the_log_reveals_gradually_rather_than_all_at_once() {
        let mut app = app();
        app.apply(Action::MapEnter);
        app.apply(Action::AddComponent(0));
        app.apply(Action::Cast);

        let total = app.combat.as_ref().unwrap().log.len();
        assert!(total > 1, "casting should produce several log entries");
        assert_eq!(app.visible_log().len(), 0, "nothing revealed yet");

        app.tick();
        assert_eq!(app.visible_log().len(), 1);
        for _ in 0..total {
            app.tick();
        }
        assert_eq!(app.visible_log().len(), total, "reveal should catch up");
    }

    #[test]
    fn quitting_sets_the_flag_from_any_screen() {
        let mut app = app();
        app.apply(Action::Quit);
        assert!(app.should_quit);
    }
}
