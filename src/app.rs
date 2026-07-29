//! The shell: owns all state, routes actions, and drives the loop.

use rand::seq::IndexedRandom;

use crate::action::Action;
use crate::game::combat::{Combat, Command, Phase};
use crate::game::content::Content;
use crate::game::encounter;
use crate::game::event::{MapEvent, apply_outcome};
use crate::game::map::{NodeId, NodeKind};
use crate::game::options::{Cycle, OptionField, Options};
use crate::game::reward::{self, Reward};
use crate::game::run::Run;
use crate::game::shop::{self as shop, BuyError, Shop};
use crate::game::spell::{SPELL_SLOTS, Spell};

/// A fixed set of screens, so an enum beats dynamic dispatch: the compiler
/// points at every match that needs updating when a variant is added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Title,
    Options,
    Map,
    Combat,
    Reward,
    Shop,
    Event,
    GameOver,
}

/// Title-screen menu entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleEntry {
    Start,
    Options,
    Quit,
}

impl TitleEntry {
    pub const ALL: [TitleEntry; 3] = [TitleEntry::Start, TitleEntry::Options, TitleEntry::Quit];

    pub fn label(self) -> &'static str {
        match self {
            TitleEntry::Start => "Start a run",
            TitleEntry::Options => "Options",
            TitleEntry::Quit => "Quit",
        }
    }
}

/// A spell has been acquired but every slot is full, so the player must choose
/// what it replaces. Shared by the reward and shop screens.
#[derive(Debug, Clone)]
pub struct PendingReplacement {
    pub incoming: Spell,
    pub cursor: usize,
    /// Where to go once this resolves, or is backed out of.
    pub return_to: Screen,
}

/// State of the post-fight reward screen.
#[derive(Debug, Clone)]
pub struct RewardState {
    pub reward: Reward,
    /// Indexes the offers; the value equal to `offers.len()` is the Skip
    /// button, so Skip is always reachable even when nothing is offered.
    pub cursor: usize,
}

impl RewardState {
    pub fn new(reward: Reward) -> Self {
        Self { reward, cursor: 0 }
    }

    /// Offers plus the Skip button.
    pub fn option_count(&self) -> usize {
        self.reward.offers.len() + 1
    }

    pub fn skip_index(&self) -> usize {
        self.reward.offers.len()
    }
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
    pub title_cursor: usize,
    pub option_field: OptionField,
    pub shop_cursor: usize,
    /// Transient feedback, e.g. a refused purchase.
    pub message: Option<String>,
    /// Loop ticks since the last log entry was revealed.
    pub tick_counter: u8,
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
            title_cursor: 0,
            option_field: OptionField::MapLength,
            shop_cursor: 0,
            message: None,
            tick_counter: 0,
        }
    }
}

pub struct App {
    pub content: Content,
    pub run: Run,
    pub screen: Screen,
    pub options: Options,
    pub combat: Option<Combat>,
    pub event: Option<ActiveEvent>,
    pub reward: Option<RewardState>,
    pub shop: Option<Shop>,
    pub pending: Option<PendingReplacement>,
    pub outcome: Option<RunOutcome>,
    pub ui: UiState,
    pub should_quit: bool,
}

impl App {
    pub fn new(content: Content, seed: u64) -> Self {
        let run = Run::new(&content, seed, Options::default());
        Self {
            content,
            run,
            screen: Screen::Title,
            options: Options::default(),
            combat: None,
            event: None,
            reward: None,
            shop: None,
            pending: None,
            outcome: None,
            ui: UiState::default(),
            should_quit: false,
        }
    }

    pub fn from_content(content: Content) -> Self {
        Self::new(content, rand::random())
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

    /// Reveal pending log entries a beat at a time, at the configured pace.
    pub fn tick(&mut self) {
        let Some(combat) = &self.combat else { return };
        let pending = combat.log.len();
        if self.ui.revealed >= pending {
            self.ui.tick_counter = 0;
            return;
        }

        let per_entry = self.options.log_speed.ticks_per_entry();
        if per_entry == 0 {
            self.ui.revealed = pending;
            return;
        }

        self.ui.tick_counter = self.ui.tick_counter.saturating_add(1);
        if self.ui.tick_counter >= per_entry {
            self.ui.tick_counter = 0;
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
                self.start_run();
                return;
            }
            _ => {}
        }

        if self.pending.is_some() {
            self.replacement_action(action);
            return;
        }

        match self.screen {
            Screen::Title => self.title_action(action),
            Screen::Options => self.options_action(action),
            Screen::Map => self.map_action(action),
            Screen::Combat => self.combat_action(action),
            Screen::Reward => self.reward_action(action),
            Screen::Shop => self.shop_action(action),
            Screen::Event => self.event_action(action),
            Screen::GameOver => {}
        }
    }

    fn title_action(&mut self, action: Action) {
        let count = TitleEntry::ALL.len();
        match action {
            Action::NavUp | Action::NavLeft => {
                self.ui.title_cursor = (self.ui.title_cursor + count - 1) % count;
            }
            Action::NavDown | Action::NavRight => {
                self.ui.title_cursor = (self.ui.title_cursor + 1) % count;
            }
            Action::Confirm => match TitleEntry::ALL[self.ui.title_cursor] {
                TitleEntry::Start => self.start_run(),
                TitleEntry::Options => self.screen = Screen::Options,
                TitleEntry::Quit => self.should_quit = true,
            },
            _ => {}
        }
    }

    fn options_action(&mut self, action: Action) {
        match action {
            Action::NavUp => self.ui.option_field = self.ui.option_field.prev(),
            Action::NavDown => self.ui.option_field = self.ui.option_field.next(),
            Action::NavLeft => self.options.adjust(self.ui.option_field, false),
            Action::NavRight => self.options.adjust(self.ui.option_field, true),
            // Enter backs out; options apply to the next run started.
            Action::Confirm => self.screen = Screen::Title,
            _ => {}
        }
    }

    /// Begin a fresh run with the current options and a random seed.
    pub fn start_run(&mut self) {
        self.start_run_with_seed(rand::random());
    }

    /// Begin a fresh run from a specific seed. Keeping this separate is what
    /// lets tests and the demo replay a run exactly.
    pub fn start_run_with_seed(&mut self, seed: u64) {
        self.run = Run::new(&self.content, seed, self.options);
        self.combat = None;
        self.event = None;
        self.reward = None;
        self.shop = None;
        self.pending = None;
        self.outcome = None;
        self.ui = UiState {
            option_field: self.ui.option_field,
            ..UiState::default()
        };
        self.screen = Screen::Map;
    }

    fn reward_action(&mut self, action: Action) {
        let Some(state) = &self.reward else { return };
        let count = state.option_count();
        match action {
            Action::NavLeft | Action::NavUp => {
                if let Some(state) = &mut self.reward {
                    state.cursor = (state.cursor + count - 1) % count;
                }
            }
            Action::NavRight | Action::NavDown => {
                if let Some(state) = &mut self.reward {
                    state.cursor = (state.cursor + 1) % count;
                }
            }
            Action::Confirm => self.take_reward(),
            _ => {}
        }
    }

    fn take_reward(&mut self) {
        let Some(state) = &self.reward else { return };

        if state.cursor >= state.reward.offers.len() {
            self.advance_after_node();
            return;
        }

        let spell = state.reward.offers[state.cursor].clone();
        if self.acquire_spell(spell, Screen::Reward) {
            self.advance_after_node();
        }
    }

    /// Give the player a spell. Returns true if it landed in a free slot;
    /// false if the player now has to choose what it replaces.
    fn acquire_spell(&mut self, spell: Spell, return_to: Screen) -> bool {
        if self.run.player.spells.len() < SPELL_SLOTS {
            self.run.player.spells.push(spell);
            true
        } else {
            self.pending = Some(PendingReplacement {
                incoming: spell,
                cursor: 0,
                return_to,
            });
            false
        }
    }

    /// Shared by the reward and shop screens: pick which spell to discard.
    fn replacement_action(&mut self, action: Action) {
        let slots = self.run.player.spells.len().max(1);
        match action {
            Action::NavLeft | Action::NavUp => {
                if let Some(p) = &mut self.pending {
                    p.cursor = (p.cursor + slots - 1) % slots;
                }
            }
            Action::NavRight | Action::NavDown => {
                if let Some(p) = &mut self.pending {
                    p.cursor = (p.cursor + 1) % slots;
                }
            }
            Action::Confirm => {
                let Some(p) = self.pending.take() else { return };
                if p.cursor < self.run.player.spells.len() {
                    self.run.player.spells[p.cursor] = p.incoming;
                }
                match p.return_to {
                    // A reward is a one-off choice, so resolving it ends the node.
                    Screen::Reward => self.advance_after_node(),
                    // A shop stays open; there may be more to buy.
                    other => self.screen = other,
                }
            }
            // Back out. The spell is forfeited -- in a shop it has already been
            // paid for, which the UI warns about.
            Action::Undo | Action::Clear => {
                if let Some(p) = self.pending.take() {
                    self.screen = p.return_to;
                }
            }
            _ => {}
        }
    }

    fn shop_action(&mut self, action: Action) {
        let count = self.shop.as_ref().map_or(1, |s| s.stock.len() + 1);
        match action {
            Action::NavLeft | Action::NavUp => {
                self.ui.shop_cursor = (self.ui.shop_cursor + count - 1) % count;
            }
            Action::NavRight | Action::NavDown => {
                self.ui.shop_cursor = (self.ui.shop_cursor + 1) % count;
            }
            Action::Confirm => self.buy_selected(),
            _ => {}
        }
    }

    fn buy_selected(&mut self) {
        let index = self.ui.shop_cursor;
        let Some(shop) = self.shop.as_mut() else {
            return;
        };

        // The entry past the end of the stock is Leave.
        if index >= shop.stock.len() {
            self.advance_after_node();
            return;
        }

        let spell = shop.spell_at(index);
        match shop.buy(index, &mut self.run.player) {
            Ok(_) => self.ui.message = None,
            Err(BuyError::NotEnoughGold) => {
                self.ui.message = Some("Not enough gold.".into());
            }
            Err(BuyError::AlreadySold) => {
                self.ui.message = Some("Already sold.".into());
            }
            Err(BuyError::SlotsFull) => {
                // Paid for, but homeless: choose what it replaces.
                self.ui.message = None;
                if let Some(spell) = spell {
                    self.acquire_spell(spell, Screen::Shop);
                }
            }
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
                // Mana is a per-turn budget, so a fight always opens on a full
                // one. Without this, leftover mana from the previous fight
                // carries over and the first turn is arbitrarily crippled.
                self.run.player.refill_mana();
                self.combat = Some(Combat::new(enemies));
                self.ui.revealed = 0;
                self.ui.focus = Focus::Spells;
                self.ui.spell_cursor = 0;
                self.ui.action_cursor = ACTION_CAST;
                self.screen = Screen::Combat;
            }
            NodeKind::Shop => {
                let stock = shop::generate(
                    &self.content.spells,
                    &self.run.player.spells,
                    self.run.depth(),
                    &mut self.run.rng,
                );
                self.shop = Some(stock);
                self.ui.shop_cursor = 0;
                self.ui.message = None;
                self.screen = Screen::Shop;
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
                self.offer_reward();
            }
        }
    }

    fn offer_reward(&mut self) {
        let reward = reward::generate(
            &self.content.spells,
            &self.run.player.spells,
            self.run.depth(),
            &mut self.run.rng,
        );
        // Gold is banked immediately; only the spell is a choice.
        self.run.player.gold += reward.gold;
        self.reward = Some(RewardState::new(reward));
        self.screen = Screen::Reward;
    }

    fn advance_after_node(&mut self) {
        self.combat = None;
        self.event = None;
        self.reward = None;
        self.shop = None;
        self.pending = None;
        self.ui.message = None;
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
        let mut app = App::new(Content::load().expect("content parses"), 1);
        app.start_run_with_seed(1);
        app
    }

    /// An app sitting on the title screen, before any run has begun.
    fn fresh() -> App {
        App::new(Content::load().expect("content parses"), 1)
    }

    fn in_fight() -> App {
        let mut app = app();
        app.apply(Action::Confirm);
        app
    }

    #[test]
    fn the_game_opens_on_the_title_screen() {
        let app = fresh();
        assert_eq!(app.screen, Screen::Title);
    }

    #[test]
    fn starting_a_run_puts_you_on_the_map_choosing_where_to_begin() {
        let app = app();
        assert_eq!(app.screen, Screen::Map);
        assert!(
            app.nodes_available().len() >= 2,
            "the bottom row should offer a choice of starting nodes"
        );
        assert!(app.outcome.is_none());
    }

    #[test]
    fn the_title_menu_starts_a_run() {
        let mut app = fresh();
        app.ui.title_cursor = 0; // Start a run
        app.apply(Action::Confirm);
        assert_eq!(app.screen, Screen::Map);
    }

    #[test]
    fn the_title_menu_opens_and_closes_options() {
        let mut app = fresh();
        app.ui.title_cursor = 1; // Options
        app.apply(Action::Confirm);
        assert_eq!(app.screen, Screen::Options);
        app.apply(Action::Confirm);
        assert_eq!(
            app.screen,
            Screen::Title,
            "Enter should return to the title"
        );
    }

    #[test]
    fn the_title_menu_can_quit() {
        let mut app = fresh();
        app.ui.title_cursor = 2; // Quit
        app.apply(Action::Confirm);
        assert!(app.should_quit);
    }

    #[test]
    fn the_title_cursor_wraps() {
        let mut app = fresh();
        for _ in 0..(TitleEntry::ALL.len() * 2 + 1) {
            app.apply(Action::NavDown);
            assert!(app.ui.title_cursor < TitleEntry::ALL.len());
        }
    }

    #[test]
    fn options_change_settings_and_apply_to_the_next_run() {
        use crate::game::options::MapLength;
        let mut app = fresh();
        app.options.map_length = MapLength::Short;
        app.start_run_with_seed(1);
        let short = app.run.map.row_count();

        app.options.map_length = MapLength::Long;
        app.start_run_with_seed(1);
        assert!(
            app.run.map.row_count() > short,
            "map length option had no effect"
        );
    }

    #[test]
    fn adjusting_an_option_only_touches_the_selected_field() {
        use crate::game::options::OptionField;
        let mut app = fresh();
        app.screen = Screen::Options;
        app.ui.option_field = OptionField::Difficulty;
        let before = app.options;
        app.apply(Action::NavRight);
        assert_ne!(app.options.difficulty, before.difficulty);
        assert_eq!(app.options.map_length, before.map_length);
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

    /// Cast the current build, without depending on where focus happens to be.
    fn cast(app: &mut App) {
        app.ui.focus = Focus::Actions;
        app.ui.action_cursor = ACTION_CAST;
        app.apply(Action::Confirm);
    }

    /// Kill everything in the current fight.
    fn win_fight(app: &mut App) {
        app.run.player.mana = 99;
        let count = app.combat.as_ref().expect("in a fight").enemies.len();
        for enemy in &mut app.combat.as_mut().unwrap().enemies {
            enemy.hp = 1;
        }
        for i in 0..count {
            if app.screen != Screen::Combat {
                break;
            }
            app.combat.as_mut().unwrap().target = i;
            app.run.player.mana = 99;
            app.apply(Action::AddComponent(0));
            cast(app);
        }
    }

    /// Regression: mana is a per-turn budget, so leftover from the previous
    /// fight must not carry into the next one.
    #[test]
    fn a_fight_always_opens_on_full_mana() {
        let mut app = app();
        app.run.player.mana = 1;
        app.apply(Action::Confirm);
        assert_eq!(
            app.run.player.mana, app.run.player.max_mana,
            "entering a fight should refill mana"
        );
    }

    #[test]
    fn mana_refills_at_the_end_of_every_turn() {
        let mut app = in_fight();
        app.run.player.mana = 0;
        app.apply(Action::EndTurn);
        if app.screen == Screen::Combat {
            assert_eq!(app.run.player.mana, app.run.player.max_mana);
        }
    }

    #[test]
    fn winning_a_fight_offers_a_reward_and_banks_gold() {
        let mut app = in_fight();
        win_fight(&mut app);
        assert_eq!(app.screen, Screen::Reward, "no reward screen after a win");
        assert!(app.run.player.gold > 0, "gold was not awarded");
        let state = app.reward.as_ref().unwrap();
        assert!(state.reward.offers.len() <= crate::game::reward::OFFER_COUNT);
    }

    #[test]
    fn taking_a_reward_spell_adds_it_and_returns_to_the_map() {
        let mut app = in_fight();
        win_fight(&mut app);
        let before = app.run.player.spells.len();
        let offered = app.reward.as_ref().unwrap().reward.offers[0].name.clone();

        app.reward.as_mut().unwrap().cursor = 0;
        app.apply(Action::Confirm);

        assert_eq!(app.screen, Screen::Map);
        assert_eq!(app.run.player.spells.len(), before + 1);
        assert!(app.run.player.spells.iter().any(|s| s.name == offered));
    }

    #[test]
    fn skipping_a_reward_changes_nothing_but_still_returns_to_the_map() {
        let mut app = in_fight();
        win_fight(&mut app);
        let before = app.run.player.spells.len();

        let skip = app.reward.as_ref().unwrap().skip_index();
        app.reward.as_mut().unwrap().cursor = skip;
        app.apply(Action::Confirm);

        assert_eq!(app.screen, Screen::Map);
        assert_eq!(app.run.player.spells.len(), before);
    }

    #[test]
    fn taking_a_spell_with_full_slots_asks_which_to_replace() {
        let mut app = in_fight();
        win_fight(&mut app);

        // Fill every slot so the reward has nowhere to go.
        let filler = app.run.player.spells[0].clone();
        while app.run.player.spells.len() < SPELL_SLOTS {
            app.run.player.spells.push(filler.clone());
        }

        let offered = app.reward.as_ref().unwrap().reward.offers[0].name.clone();
        app.reward.as_mut().unwrap().cursor = 0;
        app.apply(Action::Confirm);

        assert_eq!(app.screen, Screen::Reward, "should stay to pick a slot");
        assert!(app.pending.is_some());

        // Replace the second slot.
        app.apply(Action::NavRight);
        app.apply(Action::Confirm);

        assert_eq!(app.screen, Screen::Map);
        assert_eq!(
            app.run.player.spells.len(),
            SPELL_SLOTS,
            "slots stay capped"
        );
        assert_eq!(app.run.player.spells[1].name, offered);
    }

    #[test]
    fn backing_out_of_a_replacement_returns_to_the_offers() {
        let mut app = in_fight();
        win_fight(&mut app);
        let filler = app.run.player.spells[0].clone();
        while app.run.player.spells.len() < SPELL_SLOTS {
            app.run.player.spells.push(filler.clone());
        }

        app.reward.as_mut().unwrap().cursor = 0;
        app.apply(Action::Confirm);
        assert!(app.pending.is_some());

        app.apply(Action::Undo);
        assert!(
            app.pending.is_none(),
            "backspace should return to the offers"
        );
        assert_eq!(app.screen, Screen::Reward);
    }

    #[test]
    fn the_reward_cursor_wraps_over_offers_and_skip() {
        let mut app = in_fight();
        win_fight(&mut app);
        let count = app.reward.as_ref().unwrap().option_count();
        for _ in 0..(count * 2 + 1) {
            app.apply(Action::NavRight);
            assert!(app.reward.as_ref().unwrap().cursor < count);
        }
    }

    /// Drop the player into a shop without needing a map seed that has one.
    fn in_shop() -> App {
        let mut app = app();
        let stock = shop::generate(
            &app.content.spells,
            &app.run.player.spells,
            0,
            &mut app.run.rng,
        );
        app.shop = Some(stock);
        app.screen = Screen::Shop;
        app.ui.shop_cursor = 0;
        app
    }

    #[test]
    fn a_shop_node_on_the_map_opens_the_shop() {
        // Row 0 is always a fight, so look one row up for a shop.
        for seed in 0..300u64 {
            let mut app = app();
            app.start_run_with_seed(seed);
            app.apply(Action::Confirm);
            // Skip past the opening fight without playing it.
            app.combat = None;
            app.screen = Screen::Map;

            let shop_choice = app
                .nodes_available()
                .into_iter()
                .position(|id| app.run.map.node(id).kind == NodeKind::Shop);

            let Some(index) = shop_choice else { continue };
            app.ui.map_cursor = index;
            app.apply(Action::Confirm);

            assert_eq!(app.screen, Screen::Shop);
            assert!(app.shop.is_some(), "shop stock was not generated");
            assert!(
                !app.shop.as_ref().unwrap().stock.is_empty(),
                "an empty shop would strand the player"
            );
            return;
        }
        panic!("no shop node appeared in 300 seeds -- generation may be broken");
    }

    #[test]
    fn buying_in_a_shop_spends_gold() {
        let mut app = in_shop();
        app.run.player.gold = 1000;
        let price = app.shop.as_ref().unwrap().stock[0].price;
        app.ui.shop_cursor = 0;
        app.apply(Action::Confirm);
        assert_eq!(app.run.player.gold, 1000 - price);
        assert!(app.shop.as_ref().unwrap().stock[0].sold);
    }

    #[test]
    fn buying_without_gold_reports_it_and_changes_nothing() {
        let mut app = in_shop();
        app.run.player.gold = 0;
        app.ui.shop_cursor = 0;
        app.apply(Action::Confirm);
        assert_eq!(app.run.player.gold, 0);
        assert!(!app.shop.as_ref().unwrap().stock[0].sold);
        assert!(app.ui.message.is_some(), "the refusal should be explained");
        assert_eq!(
            app.screen,
            Screen::Shop,
            "a refusal must not close the shop"
        );
    }

    #[test]
    fn leaving_a_shop_returns_to_the_map() {
        let mut app = in_shop();
        app.ui.shop_cursor = app.shop.as_ref().unwrap().stock.len();
        app.apply(Action::Confirm);
        assert_eq!(app.screen, Screen::Map);
        assert!(app.shop.is_none());
    }

    #[test]
    fn buying_a_spell_with_full_slots_asks_what_to_replace_and_keeps_the_shop_open() {
        let mut app = in_shop();
        app.run.player.gold = 1000;
        let filler = app.run.player.spells[0].clone();
        while app.run.player.spells.len() < SPELL_SLOTS {
            app.run.player.spells.push(filler.clone());
        }

        let index = app
            .shop
            .as_ref()
            .unwrap()
            .stock
            .iter()
            .position(|e| matches!(e.item, crate::game::shop::ShopItem::Spell(_)));
        let Some(index) = index else { return };

        app.ui.shop_cursor = index;
        app.apply(Action::Confirm);
        assert!(app.pending.is_some(), "should ask what to replace");

        app.apply(Action::Confirm);
        assert!(app.pending.is_none());
        assert_eq!(
            app.screen,
            Screen::Shop,
            "a shop stays open after a purchase resolves"
        );
        assert_eq!(app.run.player.spells.len(), SPELL_SLOTS);
    }

    #[test]
    fn the_shop_cursor_wraps_over_stock_and_leave() {
        let mut app = in_shop();
        let count = app.shop.as_ref().unwrap().stock.len() + 1;
        for _ in 0..(count * 2 + 1) {
            app.apply(Action::NavDown);
            assert!(app.ui.shop_cursor < count);
        }
    }

    #[test]
    fn log_speed_changes_how_fast_the_log_reveals() {
        use crate::game::options::LogSpeed;

        let reveal_after_one_tick = |speed| {
            let mut app = in_fight();
            app.options.log_speed = speed;
            app.apply(Action::AddComponent(0));
            cast(&mut app);
            app.tick();
            app.visible_log().len()
        };

        let instant = reveal_after_one_tick(LogSpeed::Instant);
        let normal = reveal_after_one_tick(LogSpeed::Normal);
        let slow = reveal_after_one_tick(LogSpeed::Slow);

        assert!(
            instant > normal,
            "instant should dump the whole log at once"
        );
        assert_eq!(normal, 1, "normal reveals one entry per tick");
        assert_eq!(slow, 0, "slow takes several ticks for the first entry");
    }
}
