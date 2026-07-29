//! Status effects. Deliberately shallow -- the depth in this game comes from
//! combination, not from status bookkeeping.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum Status {
    /// Damage at end of round.
    Burned,
    /// Incoming Shock damage is doubled.
    Wet,
    /// Skips its next turn.
    Frozen,
    /// Attacks may miss.
    Blind,
    /// Damage at end of round, ignores shields.
    Poisoned,
}

impl Status {
    pub fn name(self) -> &'static str {
        match self {
            Status::Burned => "Burned",
            Status::Wet => "Wet",
            Status::Frozen => "Frozen",
            Status::Blind => "Blind",
            Status::Poisoned => "Poisoned",
        }
    }

    /// Damage dealt at the end of each round while this status is active.
    pub fn tick_damage(self) -> u16 {
        match self {
            Status::Burned => 2,
            Status::Poisoned => 1,
            _ => 0,
        }
    }
}

/// A status currently on an entity, with its remaining duration in rounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveStatus {
    pub status: Status,
    pub rounds: u8,
}

/// The set of statuses on one entity.
///
/// Kept sorted by `Status` so iteration order is deterministic, which matters
/// because tick damage is applied in iteration order and tests snapshot it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Statuses(Vec<ActiveStatus>);

impl Statuses {
    pub fn new() -> Self {
        Self::default()
    }

    /// Applying a status that is already present refreshes it to whichever
    /// duration is longer, rather than stacking a second copy.
    pub fn apply(&mut self, status: Status, rounds: u8) {
        if rounds == 0 {
            return;
        }
        match self.0.iter_mut().find(|a| a.status == status) {
            Some(existing) => existing.rounds = existing.rounds.max(rounds),
            None => {
                self.0.push(ActiveStatus { status, rounds });
                self.0.sort_by_key(|a| a.status);
            }
        }
    }

    pub fn has(&self, status: Status) -> bool {
        self.0.iter().any(|a| a.status == status)
    }

    pub fn remove(&mut self, status: Status) {
        self.0.retain(|a| a.status != status);
    }

    pub fn iter(&self) -> impl Iterator<Item = &ActiveStatus> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Total damage from ticking statuses this round.
    pub fn tick_damage(&self) -> u16 {
        self.0.iter().map(|a| a.status.tick_damage()).sum()
    }

    /// Decrement all durations, dropping any that expire.
    pub fn tick(&mut self) {
        for a in &mut self.0 {
            a.rounds = a.rounds.saturating_sub(1);
        }
        self.0.retain(|a| a.rounds > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_refreshes_to_longer_duration_rather_than_stacking() {
        let mut s = Statuses::new();
        s.apply(Status::Burned, 2);
        s.apply(Status::Burned, 4);
        assert_eq!(s.iter().count(), 1);
        assert_eq!(s.iter().next().unwrap().rounds, 4);

        // A shorter application does not shorten an existing one.
        s.apply(Status::Burned, 1);
        assert_eq!(s.iter().next().unwrap().rounds, 4);
    }

    #[test]
    fn statuses_expire_after_their_duration() {
        let mut s = Statuses::new();
        s.apply(Status::Wet, 2);
        s.tick();
        assert!(s.has(Status::Wet));
        s.tick();
        assert!(!s.has(Status::Wet));
        assert!(s.is_empty());
    }

    #[test]
    fn applying_zero_rounds_is_a_no_op() {
        let mut s = Statuses::new();
        s.apply(Status::Frozen, 0);
        assert!(s.is_empty());
    }

    #[test]
    fn iteration_order_is_deterministic() {
        let mut a = Statuses::new();
        a.apply(Status::Poisoned, 1);
        a.apply(Status::Burned, 1);

        let mut b = Statuses::new();
        b.apply(Status::Burned, 1);
        b.apply(Status::Poisoned, 1);

        let order = |s: &Statuses| s.iter().map(|x| x.status).collect::<Vec<_>>();
        assert_eq!(order(&a), order(&b));
    }

    #[test]
    fn tick_damage_sums_over_active_statuses() {
        let mut s = Statuses::new();
        s.apply(Status::Burned, 1); // 2
        s.apply(Status::Poisoned, 1); // 1
        s.apply(Status::Wet, 1); // 0
        assert_eq!(s.tick_damage(), 3);
    }
}
