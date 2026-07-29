//! A roguelike spell-battler for the terminal.
//!
//! The crate is split along one boundary that everything else follows:
//!
//! - [`game`] is the pure simulation. No `ratatui`, no IO, no async. If a type
//!   in there ever needs a terminal, something has gone wrong.
//! - [`app`], [`input`] and [`ui`] are the shell that draws it and feeds it input.
//!
//! See `docs/ARCHITECTURE.md`.

pub mod action;
pub mod app;
pub mod game;
pub mod input;
pub mod ui;
