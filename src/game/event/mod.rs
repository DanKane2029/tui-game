//! Map events: a prompt, some choices, and an outcome per choice.
//!
//! Outcomes compose from a small fixed vocabulary, so new events are pure data
//! and need no new code.

use rand::Rng;
use rand::seq::IndexedRandom;
use serde::Deserialize;

use crate::game::entity::Player;
use crate::game::spell::{SPELL_SLOTS, Spell};

#[derive(Debug, Clone, Deserialize)]
pub struct MapEvent {
    pub name: String,
    pub prompt: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub text: String,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Outcome {
    Nothing,
    Damage(u16),
    Heal(u16),
    RaiseMaxHp(u16),
    RaiseMaxMana(u8),
    /// Adds a random spell from the pool, if there is a free slot.
    GainSpell,
}

/// Apply an outcome and describe what happened, for display.
pub fn apply_outcome(
    player: &mut Player,
    outcome: Outcome,
    spell_pool: &[Spell],
    rng: &mut impl Rng,
) -> String {
    match outcome {
        Outcome::Nothing => "You move on.".to_string(),
        Outcome::Damage(n) => {
            player.take_damage(n);
            format!("You take {n} damage.")
        }
        Outcome::Heal(n) => {
            player.heal(n);
            format!("You recover {n} health.")
        }
        Outcome::RaiseMaxHp(n) => {
            player.max_hp += n;
            player.hp += n;
            format!("Your maximum health rises by {n}.")
        }
        Outcome::RaiseMaxMana(n) => {
            player.max_mana = player.max_mana.saturating_add(n);
            player.refill_mana();
            format!("Your maximum mana rises by {n}.")
        }
        Outcome::GainSpell => {
            if player.spells.len() >= SPELL_SLOTS {
                return "You have no free spell slots.".to_string();
            }
            // Only offer spells the player does not already carry.
            let known: Vec<&str> = player.spells.iter().map(|s| s.name.as_str()).collect();
            let candidates: Vec<&Spell> = spell_pool
                .iter()
                .filter(|s| !known.contains(&s.name.as_str()))
                .collect();
            match candidates.choose(rng) {
                Some(spell) => {
                    player.spells.push((*spell).clone());
                    format!("You learn {}.", spell.name)
                }
                None => "There is nothing here you do not already know.".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::element::Element;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn pool() -> Vec<Spell> {
        vec![
            Spell {
                name: "Ember".into(),
                mana_cost: 1,
                element: Element::Flame,
                power: 3,
                art: vec![],
                blurb: String::new(),
            },
            Spell {
                name: "Douse".into(),
                mana_cost: 1,
                element: Element::Water,
                power: 2,
                art: vec![],
                blurb: String::new(),
            },
        ]
    }

    #[test]
    fn gaining_a_spell_never_duplicates_one_you_have() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut player = Player::new(vec![pool()[0].clone()]);
        apply_outcome(&mut player, Outcome::GainSpell, &pool(), &mut rng);
        assert_eq!(player.spells.len(), 2);
        assert_ne!(player.spells[0].name, player.spells[1].name);
    }

    #[test]
    fn gaining_a_spell_respects_the_slot_limit() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut player = Player::new(vec![pool()[0].clone(); SPELL_SLOTS]);
        let msg = apply_outcome(&mut player, Outcome::GainSpell, &pool(), &mut rng);
        assert_eq!(player.spells.len(), SPELL_SLOTS);
        assert!(msg.contains("no free spell slots"));
    }

    #[test]
    fn damage_cannot_underflow() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut player = Player::new(vec![]);
        apply_outcome(&mut player, Outcome::Damage(9999), &pool(), &mut rng);
        assert_eq!(player.hp, 0);
    }
}
