//! Post-fight rewards: gold, plus a choice of one new spell from three.
//!
//! Offers exclude spells the player already carries, so a reward is never a
//! duplicate of something in hand.

use rand::Rng;
use rand::seq::IndexedRandom;

use crate::game::spell::Spell;

/// How many spells are offered after a fight.
pub const OFFER_COUNT: usize = 3;

#[derive(Debug, Clone)]
pub struct Reward {
    pub gold: u32,
    /// Up to [`OFFER_COUNT`] spells. Can be shorter, or empty, once the pool
    /// is exhausted.
    pub offers: Vec<Spell>,
}

/// Build a reward for clearing a fight at `depth`.
pub fn generate(pool: &[Spell], known: &[Spell], depth: usize, rng: &mut impl Rng) -> Reward {
    let known_names: Vec<&str> = known.iter().map(|s| s.name.as_str()).collect();

    let candidates: Vec<&Spell> = pool
        .iter()
        .filter(|s| !known_names.contains(&s.name.as_str()))
        .collect();

    let offers: Vec<Spell> = candidates
        .sample(rng, OFFER_COUNT)
        .map(|s| (*s).clone())
        .collect();

    Reward {
        gold: 10 + (depth as u32) * 5,
        offers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::element::Element;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn spell(name: &str) -> Spell {
        Spell {
            name: name.into(),
            mana_cost: 1,
            element: Element::Flame,
            power: 1,
            art: vec![],
            blurb: String::new(),
        }
    }

    fn pool() -> Vec<Spell> {
        ["a", "b", "c", "d", "e"].iter().map(|n| spell(n)).collect()
    }

    #[test]
    fn offers_never_duplicate_a_spell_you_already_have() {
        let mut rng = StdRng::seed_from_u64(4);
        let known = vec![spell("a"), spell("b")];
        for _ in 0..50 {
            let reward = generate(&pool(), &known, 1, &mut rng);
            for offer in &reward.offers {
                assert!(
                    offer.name != "a" && offer.name != "b",
                    "offered {}, which the player already carries",
                    offer.name
                );
            }
        }
    }

    #[test]
    fn offers_are_distinct_from_each_other() {
        let mut rng = StdRng::seed_from_u64(9);
        for _ in 0..50 {
            let reward = generate(&pool(), &[], 1, &mut rng);
            let mut names: Vec<&str> = reward.offers.iter().map(|s| s.name.as_str()).collect();
            names.sort_unstable();
            let count = names.len();
            names.dedup();
            assert_eq!(names.len(), count, "the same spell was offered twice");
        }
    }

    #[test]
    fn at_most_three_are_offered() {
        let mut rng = StdRng::seed_from_u64(1);
        let reward = generate(&pool(), &[], 1, &mut rng);
        assert!(reward.offers.len() <= OFFER_COUNT);
    }

    /// Once the player knows everything, there is nothing left to offer. The
    /// reward screen must cope with an empty list rather than assume three.
    #[test]
    fn an_exhausted_pool_yields_gold_but_no_offers() {
        let mut rng = StdRng::seed_from_u64(1);
        let reward = generate(&pool(), &pool(), 1, &mut rng);
        assert!(reward.offers.is_empty());
        assert!(reward.gold > 0);
    }

    #[test]
    fn deeper_fights_pay_better() {
        let mut rng = StdRng::seed_from_u64(2);
        let shallow = generate(&pool(), &[], 0, &mut rng).gold;
        let deep = generate(&pool(), &[], 4, &mut rng).gold;
        assert!(deep > shallow);
    }
}
