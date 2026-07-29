//! Content loading.
//!
//! Everything is embedded with `include_str!`, so the binary runs from any
//! directory rather than only from the repository root. Parsing happens once
//! at startup and returns a `Result` naming the file, instead of panicking.

use color_eyre::eyre::{Context, Result};

use crate::game::combat::rules::Rules;
use crate::game::entity::EnemyKind;
use crate::game::event::MapEvent;
use crate::game::spell::Spell;

const SPELLS: &str = include_str!("../../../res/spells.ron");
const RULES: &str = include_str!("../../../res/rules.ron");
const ENEMIES: &str = include_str!("../../../res/enemies.ron");
const EVENTS: &str = include_str!("../../../res/events.ron");

#[derive(Debug, Clone)]
pub struct Content {
    pub spells: Vec<Spell>,
    pub rules: Rules,
    pub enemies: Vec<EnemyKind>,
    pub events: Vec<MapEvent>,
}

impl Content {
    pub fn load() -> Result<Self> {
        Ok(Self {
            spells: ron::from_str(SPELLS).wrap_err("res/spells.ron")?,
            rules: ron::from_str(RULES).wrap_err("res/rules.ron")?,
            enemies: ron::from_str(ENEMIES).wrap_err("res/enemies.ron")?,
            events: ron::from_str(EVENTS).wrap_err("res/events.ron")?,
        })
    }

    /// The spells a new run starts with.
    pub fn starting_spells(&self) -> Vec<Spell> {
        self.spells.iter().take(3).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::combat::incantation::resolve;
    use crate::game::element::Element;

    fn content() -> Content {
        Content::load().expect("shipped content must parse")
    }

    #[test]
    fn all_content_files_parse() {
        let c = content();
        assert!(!c.spells.is_empty());
        assert!(!c.enemies.is_empty());
        assert!(!c.events.is_empty());
        assert!(!c.rules.interactions.is_empty());
    }

    /// The regression guard for the exact bug the prototype shipped, where
    /// res/enemies.ron carried fields the Enemy struct did not have.
    #[test]
    fn no_component_spell_uses_a_fusion_only_element() {
        for spell in content().spells {
            assert!(
                !spell.element.is_fusion(),
                "{} carries {:?}, which should only ever be produced by a fusion",
                spell.name,
                spell.element
            );
        }
    }

    #[test]
    fn every_spell_is_castable_on_its_own() {
        let c = content();
        for spell in &c.spells {
            let resolved = resolve(std::slice::from_ref(spell), &c.rules)
                .unwrap_or_else(|| panic!("{} did not resolve", spell.name));
            assert!(
                resolved.damage > 0,
                "{} resolves to zero damage",
                spell.name
            );
        }
    }

    #[test]
    fn every_pair_of_spells_resolves_to_something_named() {
        // Not asserting on the name itself -- just that no combination panics
        // or produces an empty name.
        let c = content();
        for a in &c.spells {
            for b in &c.spells {
                let bag = [a.clone(), b.clone()];
                let r = resolve(&bag, &c.rules)
                    .unwrap_or_else(|| panic!("{} + {} did not resolve", a.name, b.name));
                assert!(
                    !r.name.is_empty(),
                    "{} + {} produced no name",
                    a.name,
                    b.name
                );
            }
        }
    }

    #[test]
    fn every_event_offers_at_least_one_choice() {
        for event in content().events {
            assert!(
                !event.choices.is_empty(),
                "event {:?} has no choices, so it would soft-lock the run",
                event.name
            );
        }
    }

    #[test]
    fn every_enemy_can_actually_threaten_the_player() {
        for enemy in content().enemies {
            assert!(enemy.max_hp > 0, "{} has no health", enemy.name);
            assert!(enemy.power > 0, "{} deals no damage", enemy.name);
            assert!(
                enemy.difficulty > 0,
                "{} has no difficulty weight",
                enemy.name
            );
        }
    }

    #[test]
    fn fusion_rules_only_produce_fusion_elements() {
        use crate::game::combat::rules::Interaction;
        for ((a, b), interaction) in &content().rules.interactions {
            if let Interaction::Fuse(out) = interaction {
                assert!(
                    out.is_fusion(),
                    "({a:?}, {b:?}) fuses into {out:?}, which is a base element"
                );
                assert_ne!(a, b, "an element cannot fuse with itself");
            }
        }
    }

    #[test]
    fn starting_spells_fit_in_the_available_slots() {
        let c = content();
        assert!(!c.starting_spells().is_empty());
        assert!(c.starting_spells().len() <= crate::game::spell::SPELL_SLOTS);
    }

    /// Five spell cards sit side by side, so on an 80-column terminal each has
    /// 14 usable columns. Anything wider is silently clipped on screen, which
    /// is the kind of thing nobody notices until a screenshot.
    #[test]
    fn spell_art_and_blurbs_fit_a_card_at_eighty_columns() {
        const USABLE: usize = 14;
        for spell in content().spells {
            assert!(
                spell.blurb.chars().count() <= USABLE,
                "{}: blurb {:?} is {} cols, over the {USABLE} that fit",
                spell.name,
                spell.blurb,
                spell.blurb.chars().count()
            );
            assert!(
                spell.art.len() <= 3,
                "{}: art is {} lines; cards have room for 3",
                spell.name,
                spell.art.len()
            );
            for row in &spell.art {
                assert!(
                    row.chars().count() <= USABLE,
                    "{}: art row {row:?} is {} cols, over the {USABLE} that fit",
                    spell.name,
                    row.chars().count()
                );
            }
        }
    }

    #[test]
    fn every_spell_has_art_and_a_blurb() {
        for spell in content().spells {
            assert!(!spell.art.is_empty(), "{} has no art", spell.name);
            assert!(!spell.blurb.is_empty(), "{} has no blurb", spell.name);
        }
    }

    #[test]
    fn the_headline_combination_from_the_design_doc_still_holds() {
        // Ember + Ember + Gust should turn a single-target spell into one that
        // hits everything, at reduced power. If this breaks, the docs are lying.
        let c = content();
        let by_name = |n: &str| c.spells.iter().find(|s| s.name == n).unwrap().clone();
        let bag = [by_name("Ember"), by_name("Ember"), by_name("Gust")];

        let r = resolve(&bag, &c.rules).unwrap();
        assert_eq!(r.name, "Firestorm");
        assert_eq!(r.element, Element::Flame);
        assert_eq!(r.targeting, crate::game::combat::rules::Targeting::All);
    }
}
