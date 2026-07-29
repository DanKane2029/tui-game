//! The shop: somewhere for gold to go.
//!
//! Stock is generated per visit from the spell pool plus a few services, so
//! adding a spell to `res/spells.ron` makes it purchasable with no extra code.

use rand::Rng;
use rand::seq::IndexedRandom;

use crate::game::entity::Player;
use crate::game::spell::Spell;

#[derive(Debug, Clone)]
pub enum ShopItem {
    Spell(Spell),
    Heal(u16),
    RaiseMaxHp(u16),
    RaiseMaxMana(u8),
}

impl ShopItem {
    pub fn name(&self) -> String {
        match self {
            ShopItem::Spell(s) => s.name.clone(),
            ShopItem::Heal(n) => format!("Poultice (+{n} HP)"),
            ShopItem::RaiseMaxHp(n) => format!("Vitality (+{n} max HP)"),
            ShopItem::RaiseMaxMana(n) => format!("Attunement (+{n} max MP)"),
        }
    }

    pub fn description(&self) -> String {
        match self {
            ShopItem::Spell(s) => format!(
                "{} · pow {} · {} MP",
                s.element.name(),
                s.power,
                s.mana_cost
            ),
            ShopItem::Heal(_) => "Restores health now".into(),
            ShopItem::RaiseMaxHp(_) => "Permanent for this run".into(),
            ShopItem::RaiseMaxMana(_) => "More room to combine".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StockItem {
    pub item: ShopItem,
    pub price: u32,
    pub sold: bool,
}

#[derive(Debug, Clone)]
pub struct Shop {
    pub stock: Vec<StockItem>,
}

/// Why a purchase could not go through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuyError {
    AlreadySold,
    NotEnoughGold,
    /// Bought, but every spell slot is full -- the caller must ask the player
    /// which spell it replaces.
    SlotsFull,
}

/// Build the stock for one visit.
pub fn generate(pool: &[Spell], known: &[Spell], depth: usize, rng: &mut impl Rng) -> Shop {
    let known_names: Vec<&str> = known.iter().map(|s| s.name.as_str()).collect();
    let candidates: Vec<&Spell> = pool
        .iter()
        .filter(|s| !known_names.contains(&s.name.as_str()))
        .collect();

    let markup = 1 + depth as u32 / 2;
    let mut stock: Vec<StockItem> = candidates
        .sample(rng, 2)
        .map(|spell| StockItem {
            price: 25 + u32::from(spell.mana_cost) * 10 + markup * 5,
            item: ShopItem::Spell((*spell).clone()),
            sold: false,
        })
        .collect();

    stock.push(StockItem {
        item: ShopItem::Heal(8),
        price: 20,
        sold: false,
    });
    stock.push(StockItem {
        item: ShopItem::RaiseMaxHp(4),
        price: 45,
        sold: false,
    });
    stock.push(StockItem {
        item: ShopItem::RaiseMaxMana(1),
        price: 60,
        sold: false,
    });

    Shop { stock }
}

impl Shop {
    /// Attempt to buy `index`.
    ///
    /// Gold is only ever deducted when the purchase actually lands. A spell
    /// bought with no free slot returns [`BuyError::SlotsFull`] *after*
    /// charging, because the caller then runs the replacement flow -- the item
    /// is genuinely bought, it just needs a home.
    pub fn buy(&mut self, index: usize, player: &mut Player) -> Result<Option<Spell>, BuyError> {
        let Some(entry) = self.stock.get(index) else {
            return Err(BuyError::AlreadySold);
        };
        if entry.sold {
            return Err(BuyError::AlreadySold);
        }
        if player.gold < entry.price {
            return Err(BuyError::NotEnoughGold);
        }

        let price = entry.price;
        let item = entry.item.clone();

        match item {
            ShopItem::Spell(spell) => {
                if player.spells.len() >= crate::game::spell::SPELL_SLOTS {
                    player.gold -= price;
                    self.stock[index].sold = true;
                    return Err(BuyError::SlotsFull);
                }
                player.gold -= price;
                self.stock[index].sold = true;
                player.spells.push(spell);
                Ok(None)
            }
            ShopItem::Heal(n) => {
                player.gold -= price;
                self.stock[index].sold = true;
                player.heal(n);
                Ok(None)
            }
            ShopItem::RaiseMaxHp(n) => {
                player.gold -= price;
                self.stock[index].sold = true;
                player.max_hp += n;
                player.hp += n;
                Ok(None)
            }
            ShopItem::RaiseMaxMana(n) => {
                player.gold -= price;
                self.stock[index].sold = true;
                player.max_mana = player.max_mana.saturating_add(n);
                player.refill_mana();
                Ok(None)
            }
        }
    }

    /// The spell an entry holds, if it is one.
    pub fn spell_at(&self, index: usize) -> Option<Spell> {
        match self.stock.get(index).map(|e| &e.item) {
            Some(ShopItem::Spell(s)) => Some(s.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::element::Element;
    use crate::game::spell::SPELL_SLOTS;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn spell(name: &str) -> Spell {
        Spell {
            name: name.into(),
            mana_cost: 1,
            element: Element::Flame,
            power: 2,
            art: vec![],
            blurb: String::new(),
        }
    }

    fn pool() -> Vec<Spell> {
        ["a", "b", "c", "d"].iter().map(|n| spell(n)).collect()
    }

    fn shop() -> Shop {
        generate(&pool(), &[], 1, &mut StdRng::seed_from_u64(3))
    }

    #[test]
    fn stock_never_offers_a_spell_you_already_have() {
        let mut rng = StdRng::seed_from_u64(8);
        let known = vec![spell("a"), spell("b")];
        for _ in 0..40 {
            let shop = generate(&pool(), &known, 1, &mut rng);
            for entry in &shop.stock {
                if let ShopItem::Spell(s) = &entry.item {
                    assert!(s.name != "a" && s.name != "b");
                }
            }
        }
    }

    #[test]
    fn buying_without_enough_gold_changes_nothing() {
        let mut shop = shop();
        let mut player = Player::new(vec![]);
        player.gold = 0;
        assert_eq!(shop.buy(0, &mut player), Err(BuyError::NotEnoughGold));
        assert_eq!(player.gold, 0);
        assert!(
            !shop.stock[0].sold,
            "a failed purchase must not mark it sold"
        );
    }

    #[test]
    fn buying_deducts_gold_and_marks_the_entry_sold() {
        let mut shop = shop();
        let mut player = Player::new(vec![]);
        player.gold = 500;
        let price = shop.stock[0].price;

        assert!(shop.buy(0, &mut player).is_ok());
        assert_eq!(player.gold, 500 - price);
        assert!(shop.stock[0].sold);
    }

    #[test]
    fn the_same_entry_cannot_be_bought_twice() {
        let mut shop = shop();
        let mut player = Player::new(vec![]);
        player.gold = 500;
        assert!(shop.buy(0, &mut player).is_ok());
        assert_eq!(shop.buy(0, &mut player), Err(BuyError::AlreadySold));
    }

    #[test]
    fn a_spell_bought_with_full_slots_reports_it_and_still_charges() {
        let mut shop = generate(&pool(), &[], 1, &mut StdRng::seed_from_u64(3));
        let index = shop
            .stock
            .iter()
            .position(|e| matches!(e.item, ShopItem::Spell(_)))
            .expect("stock has a spell");

        let mut player = Player::new(vec![spell("x"); SPELL_SLOTS]);
        player.gold = 500;
        let price = shop.stock[index].price;

        assert_eq!(shop.buy(index, &mut player), Err(BuyError::SlotsFull));
        assert_eq!(
            player.gold,
            500 - price,
            "the item was bought, so charge for it"
        );
        assert!(shop.stock[index].sold);
        assert_eq!(player.spells.len(), SPELL_SLOTS, "slots stay capped");
    }

    #[test]
    fn services_apply_their_effect() {
        let mut player = Player::new(vec![]);
        player.gold = 1000;
        player.take_damage(10);
        let before_hp = player.hp;
        let before_max = player.max_hp;
        let before_mana = player.max_mana;

        let mut shop = shop();
        for i in 0..shop.stock.len() {
            let is_service = !matches!(shop.stock[i].item, ShopItem::Spell(_));
            if is_service {
                let _ = shop.buy(i, &mut player);
            }
        }

        assert!(player.hp > before_hp, "healing did not apply");
        assert!(player.max_hp > before_max, "max hp did not rise");
        assert!(player.max_mana > before_mana, "max mana did not rise");
    }

    #[test]
    fn every_entry_is_priced_above_zero() {
        for entry in shop().stock {
            assert!(entry.price > 0, "{} is free", entry.item.name());
        }
    }

    #[test]
    fn an_exhausted_spell_pool_still_yields_a_usable_shop() {
        let mut rng = StdRng::seed_from_u64(2);
        let shop = generate(&pool(), &pool(), 1, &mut rng);
        assert!(
            !shop.stock.is_empty(),
            "services should still be for sale even with no spells left"
        );
    }
}
