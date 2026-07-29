//! Combat: the incantation resolver, the rule table, and the turn state machine.

pub mod incantation;
pub mod resolve;
pub mod rules;

use crate::game::combat::incantation::{ResolvedSpell, resolve as resolve_incantation};
use crate::game::combat::resolve::{damage_against, enemy_damage};
use crate::game::combat::rules::{Rules, Targeting};
use crate::game::entity::{Enemy, Intent, Player};
use crate::game::spell::{SlotIndex, Spell};
use crate::game::status::Status;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    PlayerTurn,
    Won,
    Lost,
}

/// A request to change the fight. Commands can be **refused** -- that is the
/// whole reason they are separate from events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    AddComponent(SlotIndex),
    UndoComponent,
    ClearBuild,
    TargetNext,
    TargetPrev,
    Cast,
    EndTurn,
}

/// A statement of fact: something that already happened. The combat log is
/// just a list of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Cast { name: String },
    Damaged { target: String, amount: u16 },
    StatusApplied { target: String, status: Status },
    Died { name: String },
    EnemyAttacked { name: String, amount: u16 },
    StatusTicked { target: String, amount: u16 },
    TurnEnded,
    Refused(&'static str),
    Won,
    Lost,
}

#[derive(Debug, Clone)]
pub struct Combat {
    pub enemies: Vec<Enemy>,
    /// The in-progress incantation: indices into the player's spell slots.
    pub build: Vec<SlotIndex>,
    pub target: usize,
    pub phase: Phase,
    pub log: Vec<Event>,
    pub round: u32,
}

impl Combat {
    pub fn new(enemies: Vec<Enemy>) -> Self {
        Self {
            enemies,
            build: Vec::new(),
            target: 0,
            phase: Phase::PlayerTurn,
            log: Vec::new(),
            round: 1,
        }
    }

    pub fn is_over(&self) -> bool {
        matches!(self.phase, Phase::Won | Phase::Lost)
    }

    /// The components currently in the build, as actual spells.
    pub fn build_spells(&self, player: &Player) -> Vec<Spell> {
        self.build
            .iter()
            .filter_map(|&i| player.spell(i).cloned())
            .collect()
    }

    /// What the current build would cast as. Recomputed on every keystroke --
    /// cheap because resolution is pure.
    pub fn preview(&self, player: &Player, rules: &Rules) -> Option<ResolvedSpell> {
        resolve_incantation(&self.build_spells(player), rules)
    }

    /// Mana already committed to the in-progress build.
    pub fn committed_mana(&self, player: &Player) -> u8 {
        self.build_spells(player)
            .iter()
            .map(|s| u16::from(s.mana_cost))
            .sum::<u16>()
            .min(u16::from(u8::MAX)) as u8
    }

    pub fn living_enemies(&self) -> impl Iterator<Item = (usize, &Enemy)> {
        self.enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.is_alive())
    }

    /// Move the cursor to the next living enemy, wrapping. Dead enemies are
    /// never targetable.
    fn step_target(&mut self, forward: bool) {
        let living: Vec<usize> = self.living_enemies().map(|(i, _)| i).collect();
        if living.is_empty() {
            return;
        }
        let pos = living.iter().position(|&i| i == self.target).unwrap_or(0);
        let next = if forward {
            (pos + 1) % living.len()
        } else {
            (pos + living.len() - 1) % living.len()
        };
        self.target = living[next];
    }

    fn ensure_valid_target(&mut self) {
        if self.enemies.get(self.target).is_none_or(|e| !e.is_alive()) {
            // Bind first so the immutable borrow ends before the assignment.
            let first_living = self.living_enemies().map(|(i, _)| i).next();
            if let Some(i) = first_living {
                self.target = i;
            }
        }
    }

    /// Apply a command. Returns the events it produced; the same events are
    /// appended to `self.log`.
    pub fn apply(&mut self, player: &mut Player, rules: &Rules, cmd: Command) -> Vec<Event> {
        if self.is_over() {
            return vec![];
        }

        let events = match cmd {
            Command::AddComponent(slot) => self.add_component(player, slot),
            Command::UndoComponent => {
                self.build.pop();
                vec![]
            }
            Command::ClearBuild => {
                self.build.clear();
                vec![]
            }
            Command::TargetNext => {
                self.step_target(true);
                vec![]
            }
            Command::TargetPrev => {
                self.step_target(false);
                vec![]
            }
            Command::Cast => self.cast(player, rules),
            Command::EndTurn => self.end_turn(player),
        };

        self.log.extend(events.iter().cloned());
        events
    }

    fn add_component(&mut self, player: &Player, slot: SlotIndex) -> Vec<Event> {
        let Some(spell) = player.spell(slot) else {
            return vec![Event::Refused("no spell in that slot")];
        };
        let committed = self.committed_mana(player);
        if committed.saturating_add(spell.mana_cost) > player.mana {
            return vec![Event::Refused("not enough mana")];
        }
        self.build.push(slot);
        vec![]
    }

    fn cast(&mut self, player: &mut Player, rules: &Rules) -> Vec<Event> {
        let Some(spell) = self.preview(player, rules) else {
            return vec![Event::Refused("nothing to cast")];
        };
        if spell.mana_cost > player.mana {
            return vec![Event::Refused("not enough mana")];
        }

        player.mana -= spell.mana_cost;
        self.build.clear();

        let mut events = vec![Event::Cast {
            name: spell.name.clone(),
        }];

        let targets: Vec<usize> = match spell.targeting {
            Targeting::All => self.living_enemies().map(|(i, _)| i).collect(),
            Targeting::Single => {
                self.ensure_valid_target();
                self.living_enemies()
                    .map(|(i, _)| i)
                    .find(|&i| i == self.target)
                    .into_iter()
                    .collect()
            }
        };

        for i in targets {
            let amount = damage_against(&self.enemies[i], &spell);
            let enemy = &mut self.enemies[i];
            enemy.hp = enemy.hp.saturating_sub(amount);
            events.push(Event::Damaged {
                target: enemy.name.clone(),
                amount,
            });

            for &(status, rounds) in &spell.statuses {
                enemy.statuses.apply(status, rounds);
                events.push(Event::StatusApplied {
                    target: enemy.name.clone(),
                    status,
                });
            }

            if !enemy.is_alive() {
                events.push(Event::Died {
                    name: enemy.name.clone(),
                });
            }
        }

        self.ensure_valid_target();
        events.extend(self.check_outcome());
        events
    }

    fn end_turn(&mut self, player: &mut Player) -> Vec<Event> {
        let mut events = vec![Event::TurnEnded];
        self.build.clear();

        // Enemies act.
        for i in 0..self.enemies.len() {
            if !self.enemies[i].is_alive() {
                continue;
            }
            if self.enemies[i].statuses.has(Status::Frozen) {
                self.enemies[i].statuses.remove(Status::Frozen);
                continue;
            }
            let amount = enemy_damage(&self.enemies[i]);
            player.take_damage(amount);
            events.push(Event::EnemyAttacked {
                name: self.enemies[i].name.clone(),
                amount,
            });
            if !player.is_alive() {
                break;
            }
        }

        // End-of-round status damage.
        if player.is_alive() {
            for enemy in self.enemies.iter_mut().filter(|e| e.is_alive()) {
                let tick = enemy.statuses.tick_damage();
                if tick > 0 {
                    enemy.hp = enemy.hp.saturating_sub(tick);
                    events.push(Event::StatusTicked {
                        target: enemy.name.clone(),
                        amount: tick,
                    });
                    if !enemy.is_alive() {
                        events.push(Event::Died {
                            name: enemy.name.clone(),
                        });
                    }
                }
                enemy.statuses.tick();
            }

            let tick = player.statuses.tick_damage();
            if tick > 0 {
                player.take_damage(tick);
            }
            player.statuses.tick();
        }

        // Next turn.
        player.refill_mana();
        self.round += 1;
        for enemy in self.enemies.iter_mut().filter(|e| e.is_alive()) {
            enemy.intent = Intent::Attack(enemy_damage(enemy));
        }
        self.ensure_valid_target();

        events.extend(self.check_outcome());
        events
    }

    fn check_outcome(&mut self) -> Vec<Event> {
        if self.enemies.iter().all(|e| !e.is_alive()) {
            self.phase = Phase::Won;
            return vec![Event::Won];
        }
        vec![]
    }

    /// Called by the shell after any command, since the player can die from
    /// status damage as well as from being hit.
    pub fn note_player_death(&mut self) -> Vec<Event> {
        if self.phase == Phase::PlayerTurn {
            self.phase = Phase::Lost;
            self.log.push(Event::Lost);
            return vec![Event::Lost];
        }
        vec![]
    }
}
