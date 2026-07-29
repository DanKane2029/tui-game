use serde::Deserialize;

use crate::game::element::Element;
use crate::game::status::Statuses;

/// An enemy as authored in `res/enemies.ron`. Adding one is a single data
/// entry -- it enters the pool and starts appearing at the depth its
/// difficulty implies.
#[derive(Debug, Clone, Deserialize)]
pub struct EnemyKind {
    pub name: String,
    pub max_hp: u16,
    pub power: u16,
    pub element: Element,
    /// Damage subtracted from incoming hits, unless the spell pierces.
    #[serde(default)]
    pub armor: u16,
    /// Weight for encounter generation. Higher means tougher.
    pub difficulty: u8,
    /// ASCII art, one string per line.
    #[serde(default)]
    pub art: Vec<String>,
}

/// What an enemy intends to do on its next turn. Shown to the player so the
/// turn is a decision rather than a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Attack(u16),
}

impl Intent {
    pub fn label(self) -> String {
        match self {
            Intent::Attack(n) => format!("Attack {n}"),
        }
    }
}

/// A live enemy in a fight.
#[derive(Debug, Clone)]
pub struct Enemy {
    pub name: String,
    pub hp: u16,
    pub max_hp: u16,
    pub power: u16,
    pub element: Element,
    pub armor: u16,
    pub art: Vec<String>,
    pub statuses: Statuses,
    pub intent: Intent,
}

impl Enemy {
    pub fn from_kind(kind: &EnemyKind) -> Self {
        Self {
            name: kind.name.clone(),
            hp: kind.max_hp,
            max_hp: kind.max_hp,
            power: kind.power,
            element: kind.element,
            armor: kind.armor,
            art: kind.art.clone(),
            statuses: Statuses::new(),
            intent: Intent::Attack(kind.power),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}
