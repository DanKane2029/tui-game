use crate::game::spell::{SPELL_SLOTS, Spell};
use crate::game::status::Statuses;

/// The player.
///
/// There is exactly one of these in a run, owned by `Run`. Combat borrows it
/// rather than holding its own copy -- see `docs/ARCHITECTURE.md`.
#[derive(Debug, Clone)]
pub struct Player {
    pub hp: u16,
    pub max_hp: u16,
    pub mana: u8,
    pub max_mana: u8,
    /// Equipped component spells, at most [`SPELL_SLOTS`].
    pub spells: Vec<Spell>,
    pub statuses: Statuses,
    pub gold: u32,
}

impl Player {
    pub fn new(spells: Vec<Spell>) -> Self {
        Self {
            hp: 20,
            max_hp: 20,
            mana: 5,
            max_mana: 5,
            spells: spells.into_iter().take(SPELL_SLOTS).collect(),
            statuses: Statuses::new(),
            gold: 0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    /// Mana is a per-turn budget, not a run-long resource.
    pub fn refill_mana(&mut self) {
        self.mana = self.max_mana;
    }

    pub fn spell(&self, slot: usize) -> Option<&Spell> {
        self.spells.get(slot)
    }

    pub fn take_damage(&mut self, amount: u16) {
        self.hp = self.hp.saturating_sub(amount);
    }

    pub fn heal(&mut self, amount: u16) {
        self.hp = (self.hp + amount).min(self.max_hp);
    }

    /// Whether the player can still afford anything at all. Used to offer
    /// ending the turn once they are out of options.
    pub fn can_afford_anything(&self) -> bool {
        self.spells.iter().any(|s| s.mana_cost <= self.mana)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::element::Element;

    fn spell(cost: u8) -> Spell {
        Spell {
            name: "Test".into(),
            mana_cost: cost,
            element: Element::Flame,
            power: 1,
            art: vec![],
            blurb: String::new(),
        }
    }

    #[test]
    fn equipping_more_than_the_slot_count_truncates() {
        let p = Player::new(vec![spell(1); SPELL_SLOTS + 3]);
        assert_eq!(p.spells.len(), SPELL_SLOTS);
    }

    #[test]
    fn damage_saturates_at_zero_rather_than_underflowing() {
        let mut p = Player::new(vec![]);
        p.take_damage(9999);
        assert_eq!(p.hp, 0);
        assert!(!p.is_alive());
    }

    #[test]
    fn healing_does_not_exceed_max() {
        let mut p = Player::new(vec![]);
        p.take_damage(5);
        p.heal(100);
        assert_eq!(p.hp, p.max_hp);
    }

    #[test]
    fn can_afford_anything_reflects_remaining_mana() {
        let mut p = Player::new(vec![spell(3)]);
        p.mana = 3;
        assert!(p.can_afford_anything());
        p.mana = 2;
        assert!(!p.can_afford_anything());
    }
}
