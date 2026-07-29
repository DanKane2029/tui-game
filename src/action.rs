//! UI intents. Distinct from `game::combat::Command`, which is a request to
//! change the simulation and can be refused.
//!
//! Navigation is deliberately generic: the same four arrows and Enter mean
//! different things depending on the screen and which zone has focus, so the
//! whole game is playable without ever leaving the arrow keys.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Restart,

    NavLeft,
    NavRight,
    NavUp,
    NavDown,
    /// Enter / Space. What it does depends on the focused zone.
    Confirm,

    // Shortcuts. Everything below is reachable with arrows alone; these just
    // save keystrokes for players who want them.
    Undo,
    Clear,
    EndTurn,
    AddComponent(usize),
}
