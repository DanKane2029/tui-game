//! Entry point for both builds.
//!
//! The game itself lives in the library and knows nothing about either target.
//! All that differs here is which shell drives it: a terminal loop over
//! crossterm, or a browser render loop over ratzilla.

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> color_eyre::Result<()> {
    native::run()
}

#[cfg(target_arch = "wasm32")]
fn main() {
    web::run();
}
