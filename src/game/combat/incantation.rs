//! Incantation resolution.
//!
//! This is the heart of the game. The player builds a bag of component spells
//! during their turn and this turns that bag into a single spell.
//!
//! Resolution is a **pure function of an unordered multiset**. The same
//! components always produce the same spell, regardless of the order they were
//! added. That guarantee is load-bearing -- it is what lets the UI recompute
//! the preview on every keystroke, and it is checked directly by
//! `resolution_is_order_independent`.

use std::collections::BTreeMap;

use crate::game::combat::rules::{Rules, Targeting};
use crate::game::element::Element;
use crate::game::spell::Spell;
use crate::game::status::Status;

/// What a bag of components resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSpell {
    pub name: String,
    pub element: Element,
    pub tier: u8,
    pub damage: u16,
    pub targeting: Targeting,
    pub pierce: bool,
    pub statuses: Vec<(Status, u8)>,
    pub mana_cost: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Tally {
    count: u8,
    power: u16,
}

/// Resolve a bag of components into a single spell.
///
/// Returns `None` for an empty bag -- there is no such thing as a spell with
/// no components.
pub fn resolve(components: &[Spell], rules: &Rules) -> Option<ResolvedSpell> {
    if components.is_empty() {
        return None;
    }

    let mana_cost = components
        .iter()
        .map(|s| u16::from(s.mana_cost))
        .sum::<u16>()
        .min(u16::from(u8::MAX)) as u8;

    // 1. Tally by element. BTreeMap, not HashMap: iteration order must be
    //    deterministic because it decides tiebreaks.
    let mut tally: BTreeMap<Element, Tally> = BTreeMap::new();
    for spell in components {
        let entry = tally.entry(spell.element).or_default();
        entry.count = entry.count.saturating_add(1);
        entry.power = entry.power.saturating_add(spell.power);
    }

    // 2. Collapse fusable pairs until none remain. A fusion can create a new
    //    fusable pair, hence the loop.
    while let Some((a, b, fused)) = next_fusion(&tally, rules) {
        let ta = tally.remove(&a).expect("pair came from the tally");
        let tb = tally.remove(&b).expect("pair came from the tally");
        let entry = tally.entry(fused).or_default();
        entry.count = entry
            .count
            .saturating_add(ta.count)
            .saturating_add(tb.count);
        entry.power = entry
            .power
            .saturating_add(ta.power)
            .saturating_add(tb.power);
    }

    // 3. Pick the base element.
    let base = choose_base(&tally, rules);
    let base_tally = tally[&base];

    // 4. Apply every remaining element as a modifier on the base.
    //    Note their power does NOT contribute to damage -- a modifier costs
    //    mana and reshapes the spell rather than making it bigger.
    let mut damage = f32::from(base_tally.power);
    let mut targeting = Targeting::Single;
    let mut pierce = false;
    let mut added_statuses = Vec::new();

    for (&element, _) in tally.iter().filter(|&(&e, _)| e != base) {
        if let Some(m) = rules.modifier(base, element) {
            damage *= m.power;
            if let Some(t) = m.targeting {
                targeting = t;
            }
            pierce |= m.pierce;
            if let Some(s) = m.adds {
                added_statuses.push((s, 2));
            }
        }
    }

    // 5. Tier is how many base-element components went in.
    let tier = base_tally.count;

    // 6. Statuses: whatever the element applies inherently, plus modifier adds.
    let mut statuses = rules.element_statuses(base, tier);
    statuses.extend(added_statuses);
    statuses.sort_by_key(|(s, _)| *s);
    statuses.dedup_by_key(|(s, _)| *s);

    // 7. Name it.
    let name = rules
        .name(base, tier, targeting)
        .map(str::to_owned)
        .unwrap_or_else(|| fallback_name(base, tier));

    Some(ResolvedSpell {
        name,
        element: base,
        tier,
        damage: damage.round().clamp(0.0, f32::from(u16::MAX)) as u16,
        targeting,
        pierce,
        statuses,
        mana_cost,
    })
}

/// The first fusable pair, scanning in sorted element order so the choice is
/// deterministic when several pairs could fuse.
fn next_fusion(
    tally: &BTreeMap<Element, Tally>,
    rules: &Rules,
) -> Option<(Element, Element, Element)> {
    let elements: Vec<Element> = tally.keys().copied().collect();
    for (i, &a) in elements.iter().enumerate() {
        for &b in &elements[i + 1..] {
            if let Some(fused) = rules.fusion(a, b) {
                return Some((a, b, fused));
            }
        }
    }
    None
}

/// The element with the greatest total power.
///
/// Ties are broken by preferring an element that appears as the *base* side of
/// a `Modify` rule against another tied element, and finally by declaration
/// order in `Element`. Deterministic in every case.
fn choose_base(tally: &BTreeMap<Element, Tally>, rules: &Rules) -> Element {
    let max_power = tally
        .values()
        .map(|t| t.power)
        .max()
        .expect("tally is non-empty");

    let tied: Vec<Element> = tally
        .iter()
        .filter(|(_, t)| t.power == max_power)
        .map(|(&e, _)| e)
        .collect();

    if let [only] = tied[..] {
        return only;
    }

    for &a in &tied {
        for &b in &tied {
            if a != b && rules.modifier(a, b).is_some() {
                return a;
            }
        }
    }

    tied[0]
}

fn fallback_name(element: Element, tier: u8) -> String {
    let numeral = match tier {
        0 | 1 => "",
        2 => " II",
        3 => " III",
        4 => " IV",
        _ => " V",
    };
    format!("{}{}", element.name(), numeral)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::combat::rules::{ElementStatus, Interaction, Modifier, NameRule};
    use itertools::Itertools;

    fn spell(name: &str, cost: u8, element: Element, power: u16) -> Spell {
        Spell {
            name: name.into(),
            mana_cost: cost,
            element,
            power,
            art: vec![],
            blurb: String::new(),
        }
    }

    fn ember() -> Spell {
        spell("Ember", 1, Element::Flame, 3)
    }
    fn douse() -> Spell {
        spell("Douse", 1, Element::Water, 2)
    }
    fn gust() -> Spell {
        spell("Gust", 1, Element::Gust, 2)
    }
    fn surge() -> Spell {
        spell("Surge", 3, Element::Shock, 5)
    }

    fn modifier(power: f32, targeting: Option<Targeting>) -> Interaction {
        Interaction::Modify(Modifier {
            power,
            targeting,
            pierce: false,
            adds: None,
        })
    }

    fn test_rules() -> Rules {
        Rules {
            interactions: vec![
                (
                    (Element::Flame, Element::Water),
                    Interaction::Fuse(Element::Steam),
                ),
                (
                    (Element::Flame, Element::Gust),
                    modifier(0.8, Some(Targeting::All)),
                ),
                ((Element::Shock, Element::Water), modifier(2.0, None)),
            ],
            names: vec![
                NameRule {
                    element: Element::Flame,
                    tier: Some(1),
                    targeting: None,
                    name: "Ember".into(),
                },
                NameRule {
                    element: Element::Flame,
                    tier: Some(2),
                    targeting: None,
                    name: "Fireball".into(),
                },
                NameRule {
                    element: Element::Flame,
                    tier: None,
                    targeting: Some(Targeting::All),
                    name: "Firestorm".into(),
                },
                NameRule {
                    element: Element::Steam,
                    tier: None,
                    targeting: None,
                    name: "Steam Burst".into(),
                },
            ],
            element_statuses: vec![ElementStatus {
                element: Element::Flame,
                min_tier: 2,
                status: Status::Burned,
                rounds: 2,
            }],
        }
    }

    #[test]
    fn empty_bag_resolves_to_nothing() {
        assert!(resolve(&[], &test_rules()).is_none());
    }

    #[test]
    fn a_single_component_is_itself() {
        let r = resolve(&[ember()], &test_rules()).unwrap();
        assert_eq!(r.name, "Ember");
        assert_eq!(r.element, Element::Flame);
        assert_eq!(r.tier, 1);
        assert_eq!(r.damage, 3);
        assert_eq!(r.targeting, Targeting::Single);
        assert!(r.statuses.is_empty(), "tier 1 Flame does not burn");
    }

    #[test]
    fn stacking_the_same_element_raises_tier_and_sums_power() {
        let r = resolve(&[ember(), ember()], &test_rules()).unwrap();
        assert_eq!(r.name, "Fireball");
        assert_eq!(r.tier, 2);
        assert_eq!(r.damage, 6);
        assert_eq!(r.statuses, vec![(Status::Burned, 2)]);
    }

    #[test]
    fn a_modifier_reshapes_the_spell_and_can_reduce_damage() {
        // The headline example from DESIGN.md: adding Gust converts the spell
        // to hit everything, at the cost of power.
        let r = resolve(&[ember(), ember(), gust()], &test_rules()).unwrap();
        assert_eq!(r.name, "Firestorm");
        assert_eq!(r.targeting, Targeting::All);
        assert_eq!(r.damage, 5, "6 * 0.8 = 4.8, rounds to 5");
        assert_eq!(r.tier, 2, "Gust is not the base, so tier is unchanged");
    }

    #[test]
    fn modifier_power_does_not_contribute_to_damage() {
        // Surge alone is 5. Douse adds 2 power but is a modifier, so the only
        // thing it does is double -- 10, not (5+2)*2.
        let r = resolve(&[surge(), douse()], &test_rules()).unwrap();
        assert_eq!(r.element, Element::Shock);
        assert_eq!(r.damage, 10);
    }

    #[test]
    fn an_even_split_fuses_instead_of_needing_a_tiebreak() {
        let r = resolve(&[ember(), douse()], &test_rules()).unwrap();
        assert_eq!(r.name, "Steam Burst");
        assert_eq!(r.element, Element::Steam);
        assert_eq!(r.damage, 5, "fusion combines the power of both");
    }

    #[test]
    fn mana_cost_is_the_sum_of_components() {
        let r = resolve(&[ember(), ember(), surge()], &test_rules()).unwrap();
        assert_eq!(r.mana_cost, 5);
    }

    #[test]
    fn base_is_the_highest_power_element() {
        let r = resolve(&[surge(), gust()], &test_rules()).unwrap();
        assert_eq!(r.element, Element::Shock, "5 power beats 2");
    }

    /// The guarantee the whole design rests on. Order inside the bag must not
    /// matter, so it is checked directly rather than hoped for.
    #[test]
    fn resolution_is_order_independent() {
        let rules = test_rules();
        let bags = [
            vec![ember(), ember(), gust()],
            vec![ember(), douse()],
            vec![surge(), douse(), gust()],
            vec![ember(), douse(), surge(), gust()],
        ];

        for bag in bags {
            let expected = resolve(&bag, &rules);
            let len = bag.len();
            for perm in bag.iter().cloned().permutations(len) {
                assert_eq!(
                    resolve(&perm, &rules),
                    expected,
                    "permutation {:?} resolved differently",
                    perm.iter().map(|s| &s.name).collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn unnamed_combinations_still_get_a_sensible_name() {
        let r = resolve(&[gust(), gust()], &test_rules()).unwrap();
        assert_eq!(r.name, "Gust II");
    }
}
