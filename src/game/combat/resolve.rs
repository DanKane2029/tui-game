//! The damage pipeline.
//!
//! Deliberately deterministic -- no RNG anywhere in combat. That makes every
//! fight reproducible in a test and means a recorded demo replays identically.

use crate::game::combat::incantation::ResolvedSpell;
use crate::game::element::Element;
use crate::game::entity::Enemy;
use crate::game::status::Status;

/// Damage a spell deals to one target, after the target's own state is
/// accounted for.
pub fn damage_against(target: &Enemy, spell: &ResolvedSpell) -> u16 {
    let mut damage = spell.damage;

    // Water conducts: Shock against a soaked target hits twice as hard.
    // This is the cross-turn half of the combination system -- soak on one
    // cast, detonate on the next.
    if spell.element == Element::Shock && target.statuses.has(Status::Wet) {
        damage = damage.saturating_mul(2);
    }

    // Armor is what makes `pierce` worth building for.
    if !spell.pierce {
        damage = damage.saturating_sub(target.armor);
    }

    damage
}

/// Damage an enemy deals, after its own statuses are accounted for.
pub fn enemy_damage(enemy: &Enemy) -> u16 {
    let mut damage = enemy.power;
    if enemy.statuses.has(Status::Blind) {
        damage /= 2;
    }
    damage
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::combat::rules::Targeting;
    use crate::game::entity::EnemyKind;

    fn enemy(armor: u16) -> Enemy {
        Enemy::from_kind(&EnemyKind {
            name: "Dummy".into(),
            max_hp: 20,
            power: 4,
            element: Element::Earth,
            armor,
            difficulty: 1,
            art: vec![],
        })
    }

    fn spell(element: Element, damage: u16, pierce: bool) -> ResolvedSpell {
        ResolvedSpell {
            name: "Test".into(),
            element,
            tier: 1,
            damage,
            targeting: Targeting::Single,
            pierce,
            statuses: vec![],
            mana_cost: 1,
        }
    }

    #[test]
    fn armor_reduces_damage() {
        let e = enemy(3);
        assert_eq!(damage_against(&e, &spell(Element::Flame, 10, false)), 7);
    }

    #[test]
    fn pierce_ignores_armor() {
        let e = enemy(3);
        assert_eq!(damage_against(&e, &spell(Element::Flame, 10, true)), 10);
    }

    #[test]
    fn armor_cannot_push_damage_below_zero() {
        let e = enemy(100);
        assert_eq!(damage_against(&e, &spell(Element::Flame, 5, false)), 0);
    }

    #[test]
    fn shock_doubles_against_a_wet_target() {
        let mut e = enemy(0);
        e.statuses.apply(Status::Wet, 2);
        assert_eq!(damage_against(&e, &spell(Element::Shock, 6, false)), 12);
        // Only Shock benefits.
        assert_eq!(damage_against(&e, &spell(Element::Flame, 6, false)), 6);
    }

    #[test]
    fn wet_doubling_happens_before_armor() {
        let mut e = enemy(4);
        e.statuses.apply(Status::Wet, 2);
        // (6 * 2) - 4, not (6 - 4) * 2
        assert_eq!(damage_against(&e, &spell(Element::Shock, 6, false)), 8);
    }

    #[test]
    fn blind_halves_an_enemys_output() {
        let mut e = enemy(0);
        assert_eq!(enemy_damage(&e), 4);
        e.statuses.apply(Status::Blind, 2);
        assert_eq!(enemy_damage(&e), 2);
    }
}
