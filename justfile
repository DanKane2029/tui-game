# Project commands. Run `just` to see this list.

default:
    @just --list

# Play the game.
run:
    cargo run

# Live check/clippy/test feedback -- run in a SECOND terminal, since the game owns the first.
watch:
    bacon clippy-all

# Lint everything, warnings are errors (same as CI).
check:
    cargo clippy --all-targets --all-features -- -D warnings

fmt:
    cargo fmt

fmt-check:
    cargo fmt --check

test:
    cargo nextest run --no-tests=pass

# Review pending UI snapshot changes after a layout edit.
snap:
    cargo insta review

# Record the README demo GIF.
demo:
    cargo build --release
    asciinema rec --overwrite -c "./target/release/incantation" assets/demo.cast
    agg --cols 100 --rows 30 assets/demo.cast assets/demo.gif

# Everything CI runs, so you can catch failures before pushing.
ci: fmt-check check test
    cargo build --release

clean:
    cargo clean
