//! Component spells -- the things that go into an incantation.
//!
//! A component is deliberately unremarkable on its own. What makes it
//! interesting is what it does to the other components in the bag.

use serde::Deserialize;

use crate::game::element::Element;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Spell {
    pub name: String,
    pub mana_cost: u8,
    pub element: Element,
    pub power: u16,
}

/// Index into the player's equipped spell slots.
pub type SlotIndex = usize;

/// How many spells the player can have equipped at once.
pub const SPELL_SLOTS: usize = 5;
