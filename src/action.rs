//! UI intents. Distinct from `game::combat::Command`, which is a request to
//! change the simulation and can be refused.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Restart,

    // Map
    MapNext,
    MapPrev,
    MapEnter,

    // Combat
    AddComponent(usize),
    Undo,
    Clear,
    TargetNext,
    TargetPrev,
    Cast,
    EndTurn,

    // Event
    ChoiceNext,
    ChoicePrev,
    ChoiceSelect,

    /// Dismiss a result and move on.
    Continue,
}
