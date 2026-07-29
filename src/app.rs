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

/// Which zone of the fight screen the arrow keys are currently driving.
/// Ordered top to bottom, matching the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Enemies,
    Actions,
    Spells,
}

impl Focus {
    fn up(self) -> Self {
        match self {
            Focus::Spells => Focus::Actions,
            Focus::Actions | Focus::Enemies => Focus::Enemies,
        }
    }

    fn down(self) -> Self {
        match self {
            Focus::Enemies => Focus::Actions,
            Focus::Actions | Focus::Spells => Focus::Spells,
        }
    }
}

/// The two things the action zone can do.
pub const ACTION_CAST: usize = 0;
pub const ACTION_END_TURN: usize = 1;
pub const ACTION_COUNT: usize = 2;

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
#[derive(Debug, Clone)]
pub struct UiState {
    pub map_cursor: usize,
    pub choice_cursor: usize,
    /// How many combat log entries are on screen. Grows over time so a turn
    /// reads as a sequence of beats rather than one instant dump.
    pub revealed: usize,
    pub focus: Focus,
    pub spell_cursor: usize,
    pub action_cursor: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            map_cursor: 0,
            choice_cursor: 0,
            revealed: 0,
            // Start on the spell row: that is where most turns begin.
            focus: Focus::Spells,
            spell_cursor: 0,
            action_cursor: ACTION_CAST,
        }
    }
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

    /// True while a result is on screen waiting to be dismissed.
    pub fn awaiting_dismiss(&self) -> bool {
        self.event.as_ref().is_some_and(|e| e.result.is_some())
    }

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
            Action::Quit => {
                self.should_quit = true;
                return;
            }
            Action::Restart if self.screen == Screen::GameOver => {
                *self = App::new(self.content.clone(), rand::random());
                return;
            }
            _ => {}
        }

        match self.screen {
            Screen::Map => self.map_action(action),
            Screen::Combat => self.combat_action(action),
            Screen::Event => self.event_action(action),
            Screen::GameOver => {}
        }
    }

    fn map_action(&mut self, action: Action) {
        let count = self.nodes_available().len().max(1);
        match action {
            Action::NavLeft | Action::NavUp => {
                self.ui.map_cursor = (self.ui.map_cursor + count - 1) % count;
            }
            Action::NavRight | Action::NavDown => {
                self.ui.map_cursor = (self.ui.map_cursor + 1) % count;
            }
            Action::Confirm => self.enter_selected_node(),
            _ => {}
        }
    }

    fn event_action(&mut self, action: Action) {
        if self.awaiting_dismiss() {
            if action == Action::Confirm {
                self.dismiss_result();
            }
            return;
        }

        let count = self
            .event
            .as_ref()
            .map_or(1, |a| a.event.choices.len().max(1));

        match action {
            Action::NavUp | Action::NavLeft => {
                self.ui.choice_cursor = (self.ui.choice_cursor + count - 1) % count;
            }
            Action::NavDown | Action::NavRight => {
                self.ui.choice_cursor = (self.ui.choice_cursor + 1) % count;
            }
            Action::Confirm => self.resolve_event_choice(),
            _ => {}
        }
    }

    fn combat_action(&mut self, action: Action) {
        match action {
            Action::NavUp => self.ui.focus = self.ui.focus.up(),
            Action::NavDown => self.ui.focus = self.ui.focus.down(),

            Action::NavLeft => match self.ui.focus {
                Focus::Enemies => self.combat_command(Command::TargetPrev),
                Focus::Actions => {
                    self.ui.action_cursor =
                        (self.ui.action_cursor + ACTION_COUNT - 1) % ACTION_COUNT;
                }
                Focus::Spells => self.move_spell_cursor(-1),
            },
            Action::NavRight => match self.ui.focus {
                Focus::Enemies => self.combat_command(Command::TargetNext),
                Focus::Actions => {
                    self.ui.action_cursor = (self.ui.action_cursor + 1) % ACTION_COUNT;
                }
                Focus::Spells => self.move_spell_cursor(1),
            },

            Action::Confirm => match self.ui.focus {
                // Picking a target drops focus back to the spells, which is
                // where the next thing you want to do almost always is.
                Focus::Enemies => self.ui.focus = Focus::Spells,
                Focus::Actions => {
                    if self.ui.action_cursor == ACTION_CAST {
                        self.combat_command(Command::Cast);
                    } else {
                        self.combat_command(Command::EndTurn);
                    }
                }
                Focus::Spells => {
                    let slot = self.ui.spell_cursor;
                    self.combat_command(Command::AddComponent(slot));
                }
            },

            Action::Undo => self.combat_command(Command::UndoComponent),
            Action::Clear => self.combat_command(Command::ClearBuild),
            Action::EndTurn => self.combat_command(Command::EndTurn),
            Action::AddComponent(slot) => {
                self.ui.spell_cursor = slot.min(self.spell_count().saturating_sub(1));
                self.combat_command(Command::AddComponent(slot));
            }
            _ => {}
        }
    }

    fn spell_count(&self) -> usize {
        self.run.player.spells.len()
    }

    fn move_spell_cursor(&mut self, delta: isize) {
        let count = self.spell_count();
        if count == 0 {
            return;
        }
        let current = self.ui.spell_cursor as isize;
        self.ui.spell_cursor = (current + delta).rem_euclid(count as isize) as usize;
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
                self.ui.focus = Focus::Spells;
                self.ui.spell_cursor = 0;
                self.ui.action_cursor = ACTION_CAST;
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
                    // No events authored: treat the node as cleared rather
                    // than stranding the player on an empty screen.
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

    fn in_fight() -> App {
        let mut app = app();
        app.apply(Action::Confirm);
        app
    }

    #[test]
    fn a_new_app_starts_on_the_map_with_one_choice() {
        let app = app();
        assert_eq!(app.screen, Screen::Map);
        assert_eq!(app.nodes_available().len(), 1);
        assert!(app.outcome.is_none());
    }

    #[test]
    fn confirming_on_the_map_starts_a_fight() {
        let app = in_fight();
        assert_eq!(app.screen, Screen::Combat);
        assert!(
            !app.combat
                .as_ref()
                .expect("fight created")
                .enemies
                .is_empty()
        );
        assert_eq!(app.ui.focus, Focus::Spells, "turns start on the spell row");
    }

    #[test]
    fn up_and_down_cycle_focus_and_stop_at_the_ends() {
        let mut app = in_fight();
        assert_eq!(app.ui.focus, Focus::Spells);
        app.apply(Action::NavUp);
        assert_eq!(app.ui.focus, Focus::Actions);
        app.apply(Action::NavUp);
        assert_eq!(app.ui.focus, Focus::Enemies);
        // Clamps rather than wrapping, so holding up never loops around.
        app.apply(Action::NavUp);
        assert_eq!(app.ui.focus, Focus::Enemies);

        app.apply(Action::NavDown);
        assert_eq!(app.ui.focus, Focus::Actions);
        app.apply(Action::NavDown);
        assert_eq!(app.ui.focus, Focus::Spells);
        app.apply(Action::NavDown);
        assert_eq!(app.ui.focus, Focus::Spells);
    }

    #[test]
    fn the_whole_turn_is_playable_with_arrows_and_enter_alone() {
        let mut app = in_fight();
        let target = app.combat.as_ref().unwrap().target;
        let before = app.combat.as_ref().unwrap().enemies[target].hp;

        // Add the focused spell twice, then move to the action row and cast.
        app.apply(Action::Confirm);
        app.apply(Action::Confirm);
        assert_eq!(app.combat.as_ref().unwrap().build.len(), 2);

        app.apply(Action::NavUp);
        assert_eq!(app.ui.focus, Focus::Actions);
        assert_eq!(app.ui.action_cursor, ACTION_CAST);
        app.apply(Action::Confirm);

        let combat = app.combat.as_ref().unwrap();
        assert!(combat.build.is_empty(), "casting clears the build");
        assert!(
            combat.enemies[target].hp < before || !combat.enemies[target].is_alive(),
            "target took no damage"
        );
    }

    #[test]
    fn the_action_row_can_end_the_turn() {
        let mut app = in_fight();
        let round = app.combat.as_ref().unwrap().round;

        app.apply(Action::NavUp);
        app.apply(Action::NavRight);
        assert_eq!(app.ui.action_cursor, ACTION_END_TURN);
        app.apply(Action::Confirm);

        if app.screen == Screen::Combat {
            assert!(app.combat.as_ref().unwrap().round > round);
        }
    }

    #[test]
    fn the_spell_cursor_wraps_and_stays_in_range() {
        let mut app = in_fight();
        let count = app.run.player.spells.len();
        assert!(count > 0);
        for _ in 0..(count * 3 + 1) {
            app.apply(Action::NavRight);
            assert!(app.ui.spell_cursor < count);
        }
        for _ in 0..(count * 3 + 1) {
            app.apply(Action::NavLeft);
            assert!(app.ui.spell_cursor < count);
        }
    }

    #[test]
    fn confirming_a_target_returns_focus_to_the_spells() {
        let mut app = in_fight();
        app.apply(Action::NavUp);
        app.apply(Action::NavUp);
        assert_eq!(app.ui.focus, Focus::Enemies);
        app.apply(Action::Confirm);
        assert_eq!(app.ui.focus, Focus::Spells);
    }

    #[test]
    fn building_an_incantation_beyond_your_mana_is_refused() {
        let mut app = in_fight();
        app.run.player.mana = 1;
        app.apply(Action::AddComponent(0));
        app.apply(Action::AddComponent(0));
        assert_eq!(app.combat.as_ref().unwrap().build.len(), 1);
    }

    #[test]
    fn dying_ends_the_run() {
        let mut app = in_fight();
        app.run.player.hp = 1;
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
        let mut app = in_fight();
        for enemy in &mut app.combat.as_mut().unwrap().enemies {
            enemy.hp = 1;
        }
        app.run.player.mana = 99;
        for _ in 0..12 {
            if app.screen != Screen::Combat {
                break;
            }
            app.apply(Action::AddComponent(0));
            app.apply(Action::EndTurn);
        }
        assert_ne!(app.screen, Screen::Combat, "fight did not resolve");
    }

    #[test]
    fn the_log_reveals_gradually_rather_than_all_at_once() {
        let mut app = in_fight();
        app.apply(Action::AddComponent(0));
        app.apply(Action::NavUp);
        app.apply(Action::Confirm);

        let total = app.combat.as_ref().unwrap().log.len();
        assert!(total > 1);
        assert_eq!(app.visible_log().len(), 0);
        app.tick();
        assert_eq!(app.visible_log().len(), 1);
        for _ in 0..total {
            app.tick();
        }
        assert_eq!(app.visible_log().len(), total);
    }

    #[test]
    fn quitting_sets_the_flag() {
        let mut app = app();
        app.apply(Action::Quit);
        assert!(app.should_quit);
    }
}
