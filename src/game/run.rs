//! Run state -- the roguelike layer that sits above individual fights.
//!
//! `Run` owns the one and only [`Player`]. Combat borrows it rather than
//! holding a copy, which is what makes the prototype's duplicate-player bug
//! impossible to reintroduce.

use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::game::content::Content;
use crate::game::entity::Player;
use crate::game::map::{Map, NodeId, NodeKind, generate};

pub const MAP_ROWS: usize = 5;

#[derive(Debug)]
pub struct Run {
    pub map: Map,
    pub player: Player,
    /// The node currently occupied. `None` before the first node is entered.
    pub position: Option<NodeId>,
    pub visited: Vec<NodeId>,
    pub seed: u64,
    pub rng: StdRng,
}

impl Run {
    pub fn new(content: &Content, seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let map = generate(MAP_ROWS, &mut rng);
        Self {
            map,
            player: Player::new(content.starting_spells()),
            position: None,
            visited: Vec::new(),
            seed,
            rng,
        }
    }

    /// Nodes the player may move to right now.
    pub fn available(&self) -> Vec<NodeId> {
        match self.position {
            None => vec![self.map.start()],
            Some(id) => self.map.node(id).next.clone(),
        }
    }

    pub fn can_enter(&self, id: NodeId) -> bool {
        self.available().contains(&id)
    }

    pub fn enter(&mut self, id: NodeId) {
        debug_assert!(self.can_enter(id), "entered an unreachable node");
        self.position = Some(id);
        self.visited.push(id);
    }

    pub fn current_kind(&self) -> Option<NodeKind> {
        self.position.map(|id| self.map.node(id).kind)
    }

    pub fn depth(&self) -> usize {
        self.position.map_or(0, |id| self.map.node(id).row)
    }

    /// Difficulty budget for a fight at the current depth.
    pub fn encounter_budget(&self) -> u8 {
        self.map.budget_for_row(self.depth())
    }

    /// True once the boss node has been cleared.
    pub fn is_complete(&self) -> bool {
        self.position == Some(self.map.boss())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content() -> Content {
        Content::load().expect("content parses")
    }

    #[test]
    fn a_new_run_offers_exactly_the_start_node() {
        let run = Run::new(&content(), 1);
        assert_eq!(run.available(), vec![run.map.start()]);
        assert_eq!(run.depth(), 0);
    }

    #[test]
    fn a_full_climb_always_reaches_the_boss() {
        // Walk the map greedily from every seed; the run must always be
        // completable, never dead-ended.
        for seed in 0..100 {
            let mut run = Run::new(&content(), seed);
            let mut steps = 0;
            while !run.is_complete() {
                let next = run.available();
                assert!(
                    !next.is_empty(),
                    "seed {seed} dead-ended at depth {}",
                    run.depth()
                );
                run.enter(next[0]);
                steps += 1;
                assert!(steps <= MAP_ROWS + 2, "seed {seed} did not terminate");
            }
            assert_eq!(run.depth(), MAP_ROWS - 1);
        }
    }

    #[test]
    fn the_encounter_budget_grows_with_depth() {
        let mut run = Run::new(&content(), 3);
        let shallow = run.encounter_budget();
        while !run.is_complete() {
            let next = run.available();
            run.enter(next[0]);
        }
        assert!(run.encounter_budget() > shallow);
    }

    #[test]
    fn the_same_seed_produces_the_same_run() {
        let a = Run::new(&content(), 12345);
        let b = Run::new(&content(), 12345);
        assert_eq!(a.map.nodes.len(), b.map.nodes.len());
        for (x, y) in a.map.nodes.iter().zip(b.map.nodes.iter()) {
            assert_eq!(x.kind, y.kind);
            assert_eq!(x.next, y.next);
        }
    }

    #[test]
    fn the_player_starts_with_spells_and_full_resources() {
        let run = Run::new(&content(), 1);
        assert!(!run.player.spells.is_empty());
        assert_eq!(run.player.hp, run.player.max_hp);
        assert_eq!(run.player.mana, run.player.max_mana);
    }
}
