//! Encounter generation.
//!
//! Fights are assembled from a weighted pool against a difficulty budget, so
//! adding an enemy is a single data entry -- it enters the pool and starts
//! appearing at whatever depth its weight implies.

use rand::Rng;
use rand::seq::IndexedRandom;

use crate::game::entity::{Enemy, EnemyKind};

/// Never put more than this many enemies on screen at once -- the fight panel
/// stops being readable beyond it.
pub const MAX_ENEMIES: usize = 3;

/// Draw enemies from `pool` until `budget` is spent.
///
/// Always returns at least one enemy, even if the budget is too small to
/// afford anything, so a fight node can never be empty.
pub fn generate(pool: &[EnemyKind], budget: u8, rng: &mut impl Rng) -> Vec<Enemy> {
    if pool.is_empty() {
        return Vec::new();
    }

    let mut remaining = i32::from(budget);
    let mut enemies = Vec::new();

    while enemies.len() < MAX_ENEMIES {
        let affordable: Vec<&EnemyKind> = pool
            .iter()
            .filter(|k| i32::from(k.difficulty) <= remaining)
            .collect();

        let Some(kind) = affordable.choose(rng) else {
            break;
        };
        remaining -= i32::from(kind.difficulty);
        enemies.push(Enemy::from_kind(kind));
    }

    if enemies.is_empty() {
        // Budget too small for anything in the pool: use the cheapest enemy
        // rather than returning a fight with nothing in it.
        let weakest = pool
            .iter()
            .min_by_key(|k| k.difficulty)
            .expect("pool is non-empty");
        enemies.push(Enemy::from_kind(weakest));
    }

    enemies
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::element::Element;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn pool() -> Vec<EnemyKind> {
        vec![
            EnemyKind {
                name: "Bat".into(),
                max_hp: 8,
                power: 2,
                element: Element::Gust,
                armor: 0,
                difficulty: 1,
                art: vec![],
            },
            EnemyKind {
                name: "Golem".into(),
                max_hp: 24,
                power: 5,
                element: Element::Earth,
                armor: 3,
                difficulty: 4,
                art: vec![],
            },
        ]
    }

    #[test]
    fn a_fight_is_never_empty() {
        let mut rng = StdRng::seed_from_u64(7);
        for budget in 0..12 {
            let enemies = generate(&pool(), budget, &mut rng);
            assert!(!enemies.is_empty(), "budget {budget} produced no enemies");
        }
    }

    #[test]
    fn an_empty_pool_yields_an_empty_fight_rather_than_panicking() {
        let mut rng = StdRng::seed_from_u64(1);
        assert!(generate(&[], 10, &mut rng).is_empty());
    }

    #[test]
    fn enemy_count_is_capped_for_readability() {
        let mut rng = StdRng::seed_from_u64(3);
        let enemies = generate(&pool(), 200, &mut rng);
        assert!(enemies.len() <= MAX_ENEMIES);
    }

    #[test]
    fn a_bigger_budget_buys_more_or_tougher_enemies() {
        let mut rng = StdRng::seed_from_u64(11);
        let small: u32 = (0..40)
            .map(|_| {
                generate(&pool(), 2, &mut rng)
                    .iter()
                    .map(|e| u32::from(e.max_hp))
                    .sum::<u32>()
            })
            .sum();
        let large: u32 = (0..40)
            .map(|_| {
                generate(&pool(), 10, &mut rng)
                    .iter()
                    .map(|e| u32::from(e.max_hp))
                    .sum::<u32>()
            })
            .sum();
        assert!(large > small, "budget did not translate into difficulty");
    }

    #[test]
    fn enemies_start_at_full_health() {
        let mut rng = StdRng::seed_from_u64(5);
        for enemy in generate(&pool(), 8, &mut rng) {
            assert_eq!(enemy.hp, enemy.max_hp);
            assert!(enemy.is_alive());
        }
    }

    #[test]
    fn generation_is_reproducible_from_a_seed() {
        let names = |seed| {
            generate(&pool(), 8, &mut StdRng::seed_from_u64(seed))
                .iter()
                .map(|e| e.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(99), names(99));
    }
}
