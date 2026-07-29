//! The combination rule table.
//!
//! Rules are keyed on *elements*, never on specific spells. That is the whole
//! reason adding a new spell is cheap: it inherits every interaction its
//! element already has, instead of needing a recipe written against each
//! existing spell.

use serde::Deserialize;

use crate::game::element::Element;
use crate::game::status::Status;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum Targeting {
    Single,
    All,
}

impl Targeting {
    pub fn label(self) -> &'static str {
        match self {
            Targeting::Single => "single target",
            Targeting::All => "ALL enemies",
        }
    }
}

/// How one element reshapes another. Every field is optional so a rule states
/// only what it changes.
#[derive(Debug, Clone, Deserialize)]
pub struct Modifier {
    #[serde(default = "unity")]
    pub power: f32,
    #[serde(default)]
    pub targeting: Option<Targeting>,
    #[serde(default)]
    pub pierce: bool,
    #[serde(default)]
    pub adds: Option<Status>,
}

fn unity() -> f32 {
    1.0
}

#[derive(Debug, Clone, Deserialize)]
pub enum Interaction {
    /// The two elements become a third. Symmetric -- order in the table is
    /// irrelevant.
    Fuse(Element),
    /// The first element stays as the base; the second reshapes it. Order in
    /// the table expresses that relationship. It has nothing to do with the
    /// order the player pressed keys.
    Modify(Modifier),
}

/// Maps a resolved signature to a name. `None` in a field means "any".
/// More specific rules are matched first.
#[derive(Debug, Clone, Deserialize)]
pub struct NameRule {
    pub element: Element,
    #[serde(default)]
    pub tier: Option<u8>,
    #[serde(default)]
    pub targeting: Option<Targeting>,
    pub name: String,
}

impl NameRule {
    /// How many constraints this rule pins down. Used to prefer specific
    /// rules over general ones.
    fn specificity(&self) -> u8 {
        self.tier.is_some() as u8 + self.targeting.is_some() as u8
    }

    fn matches(&self, element: Element, tier: u8, targeting: Targeting) -> bool {
        self.element == element
            && self.tier.is_none_or(|t| t == tier)
            && self.targeting.is_none_or(|t| t == targeting)
    }
}

/// A status an element inherently applies once it reaches a given tier.
#[derive(Debug, Clone, Deserialize)]
pub struct ElementStatus {
    pub element: Element,
    #[serde(default = "one")]
    pub min_tier: u8,
    pub status: Status,
    pub rounds: u8,
}

fn one() -> u8 {
    1
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rules {
    pub interactions: Vec<((Element, Element), Interaction)>,
    pub names: Vec<NameRule>,
    pub element_statuses: Vec<ElementStatus>,
}

impl Rules {
    /// Fusion is symmetric: both orderings are checked.
    pub fn fusion(&self, a: Element, b: Element) -> Option<Element> {
        self.interactions.iter().find_map(|((x, y), i)| match i {
            Interaction::Fuse(out) if (*x == a && *y == b) || (*x == b && *y == a) => Some(*out),
            _ => None,
        })
    }

    /// Directional: looks up `base` being reshaped by `modifier`.
    pub fn modifier(&self, base: Element, modifier: Element) -> Option<&Modifier> {
        self.interactions.iter().find_map(|((x, y), i)| match i {
            Interaction::Modify(m) if *x == base && *y == modifier => Some(m),
            _ => None,
        })
    }

    /// The most specific authored name for this signature, if any.
    pub fn name(&self, element: Element, tier: u8, targeting: Targeting) -> Option<&str> {
        self.names
            .iter()
            .filter(|r| r.matches(element, tier, targeting))
            .max_by_key(|r| r.specificity())
            .map(|r| r.name.as_str())
    }

    /// Statuses this element applies inherently at the given tier.
    pub fn element_statuses(&self, element: Element, tier: u8) -> Vec<(Status, u8)> {
        self.element_statuses
            .iter()
            .filter(|e| e.element == element && tier >= e.min_tier)
            .map(|e| (e.status, e.rounds))
            .collect()
    }
}
