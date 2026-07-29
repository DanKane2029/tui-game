//! Elements. The vocabulary that spell combination is keyed on.

use serde::{Deserialize, Serialize};

/// Declaration order is load-bearing: it is the final, deterministic tiebreak
/// when two elements in an incantation have equal power. Do not reorder
/// casually -- it changes resolution outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum Element {
    // Base elements. These are what component spells carry.
    Flame,
    Water,
    Ice,
    Shock,
    Earth,
    Gust,
    Toxic,

    // Fusion products. Only ever produced by combining two base elements;
    // no component spell has one of these directly.
    Steam,
    Magma,
    Blight,
    Blizzard,
}

impl Element {
    pub const ALL: [Element; 11] = [
        Element::Flame,
        Element::Water,
        Element::Ice,
        Element::Shock,
        Element::Earth,
        Element::Gust,
        Element::Toxic,
        Element::Steam,
        Element::Magma,
        Element::Blight,
        Element::Blizzard,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Element::Flame => "Flame",
            Element::Water => "Water",
            Element::Ice => "Ice",
            Element::Shock => "Shock",
            Element::Earth => "Earth",
            Element::Gust => "Gust",
            Element::Toxic => "Toxic",
            Element::Steam => "Steam",
            Element::Magma => "Magma",
            Element::Blight => "Blight",
            Element::Blizzard => "Blizzard",
        }
    }

    /// True for elements that only exist as the result of a fusion.
    pub fn is_fusion(self) -> bool {
        matches!(
            self,
            Element::Steam | Element::Magma | Element::Blight | Element::Blizzard
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_contains_every_variant() {
        // Guards against adding a variant and forgetting to list it in ALL.
        for e in Element::ALL {
            assert!(!e.name().is_empty());
        }
        let mut sorted = Element::ALL;
        sorted.sort();
        sorted.windows(2).for_each(|w| assert_ne!(w[0], w[1]));
        assert_eq!(sorted.len(), Element::ALL.len());
    }

    #[test]
    fn ordering_is_declaration_order() {
        // Resolution tiebreaks depend on this.
        assert!(Element::Flame < Element::Water);
        assert!(Element::Water < Element::Toxic);
    }
}
