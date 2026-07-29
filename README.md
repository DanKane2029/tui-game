# Incantation

[![CI](https://github.com/DanKane2029/incantation/actions/workflows/ci.yml/badge.svg)](https://github.com/DanKane2029/incantation/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

**[▶ Play it in your browser](https://dankane2029.github.io/incantation/)** — no install, no download.

A roguelike spell-battler for the terminal, in Rust. You climb a branching map, and every fight
is won or lost on how you **combine spells**.

You never cast a spell. You build one.

```
┌ 1 Ember──────────┐┌ 2 Douse──────────┐┌▸3 Gust───────────┐┌ 4 ───────────────┐┌ 5 ───────────────┐
│   ^              ││   o              ││  ~~~             ││                  ││                  │
│  /^\             ││  (~)             ││ ~~~~~            ││                  ││                  │
│ (###)            ││ (~~~)            ││  ~~~             ││                  ││                  │
│Flame · pow 3     ││Water · pow 2     ││Gust · pow 2      ││                  ││                  │
│1 MP              ││1 MP              ││1 MP              ││                  ││                  │
│Stacks bigger     ││Soaks target      ││Spreads to all    ││                  ││                  │
└──────────────────┘└──────────────────┘└──────────────────┘└──────────────────┘└──────────────────┘
```

Each turn you add components to an **incantation**, and the resulting spell changes as you build.
Two Embers stack into a Fireball:

```
Build:  [Ember] + [Ember]
        FIREBALL   6 dmg · single target · 2 MP · Burned (2)
```

Add a Gust and it becomes something else entirely — **less** damage, but it hits everything:

```
Build:  [Ember] + [Ember] + [Gust]
        FIRESTORM  5 dmg · ALL enemies · 3 MP · Burned (2)
```

That tradeoff is the game. Building is a choice about what *kind* of spell you want, not a
race to stack the biggest number.

---

## The fight screen

```
┌Enemies───────────────────────────────────────────────────────────────────────┐
│                          ▸ Spider                                            │
│                           /\oo/\                                             │
│                            (..)                                              │
│                           /|||||\                                            │
│                          HP ██████ 10/10                                     │
│                          Attack 2                                            │
└──────────────────────────────────────────────────────────────────────────────┘
┌Log───────────────────────────────────────────────────────────────────────────┐
│ You cast Fireball.                                                           │
│   Spider takes 6 damage.                                                     │
│   Spider is Burned.                                                          │
└──────────────────────────────────────────────────────────────────────────────┘
┌You─────────────────────┐┌Incantation─────────────────────────────────────────┐
│HP ██████ 20/20         ││Build: [Ember] + [Ember] + [Gust]                   │
│MP ████░░ 3/5           ││FIRESTORM  5 dmg · ALL enemies · 3 MP  Burned (2)   │
│Round 1                 ││ Cast    End Turn    ↑↓ move · ←→ pick · Enter do   │
└────────────────────────┘└────────────────────────────────────────────────────┘
```

## The climb

A run is a branching map of fights, events, shops and a boss. Which node you take rules others
out — the paths are drawn so you can see what your choice costs you.

```
                   ╱─☠─╲
              ╱────  │  ────╲
        ╱─────       │       ─────╲
     ⚔──            ╱$╲            ──⚔
      ╲         ╱───   ───╲         ╱
       ─╲   ╱───           ───╲   ╱─
         ?──                  ╱──⚔
         │            ╱───────   │
         │    ╱───────           │
         ?────                   $
         │                       │
         │                       │
        [⚔]                      ⚔

     ⚔ fight    ? event    $ shop    ☠ boss
```

---

## How combination works

Order inside an incantation doesn't matter — the components are a bag, not a sequence. Rules are
keyed on **elements**, never on specific spells, which is what makes the system grow cheaply: a
new spell inherits every interaction its element already has.

There are only two kinds of rule:

```ron
// Fusion: two elements become a third.
((Flame, Water), Fuse(Steam)),
((Ice,   Gust),  Fuse(Blizzard)),

// Modify: the first stays as the base, the second reshapes it.
((Flame, Gust),  Modify((power: 0.8, targeting: Some(All)))),
((Shock, Water), Modify((power: 2.0))),          // water conducts
((Ice,   Earth), Modify((power: 1.2, pierce: true))),
```

Because you can cast more than once per turn, sequencing still matters one level up:

| What you do | Result |
|---|---|
| `{Douse, Ember}` as **one** incantation | They fuse → **Steam Burst** |
| `Douse`, then `Ember` as **two** incantations | Target is soaked, then burned |

Same two spells, same mana, different outcome.

## Playing

In a browser: **[dankane2029.github.io/incantation](https://dankane2029.github.io/incantation/)**. The
same code, compiled to WebAssembly and drawn into a DOM grid.

In a terminal:

```sh
cargo run --release
```

Wants a terminal of at least 80×26; 100×30 is comfortable.

Everything is driven with the arrow keys and Enter.

| | |
|---|---|
| **Map** | `←` `→` choose a node · `Enter` travel |
| **Fight** | `↑` `↓` move between enemies, actions and spells |
| | `←` `→` move within a row · `Enter` act · `Backspace` undo a component |
| | `1`–`5` add a spell directly (shortcut) |
| **Anywhere** | `q` quit |

## Making it yours

All content is data in `res/`, compiled into the binary. No code needed to add any of it:

| File | Holds |
|---|---|
| `spells.ron` | Component spells: cost, element, power, ASCII art |
| `rules.ron` | Fusion and modifier rules, and spell naming |
| `enemies.ron` | Enemy stats, art, difficulty weight |
| `events.ron` | Event prompts, choices, outcomes |

Add an enemy and it enters the encounter pool at whatever depth its difficulty implies. Add a
spell and it becomes buyable, offerable as a reward, and combinable with everything already
there.

## Building on it

```sh
just run      # play it
just watch    # live check/clippy/test in a second terminal (a TUI owns the first)
just test     # run the suite
just snap     # review UI snapshot changes
just ci       # everything CI runs
```

The code is split along one boundary: `src/game/` is the pure simulation — no `ratatui`, no IO,
no async, no channels — and everything else is the shell that draws it. That's what lets the
whole game be tested without a terminal.

It is also what made the browser build cheap. `game/`, `app/` and `ui/` are shared verbatim
between the two targets; only the shell differs — a blocking crossterm loop in `src/native.rs`,
a ratzilla render loop in `src/web.rs`. Key handling is shared too: both translate their own
event type into one `input::Key` and run the same mapping.

```sh
trunk serve --release      # play the web build locally at localhost:8080
```

The UI is covered by snapshot tests that render frames through ratatui's `TestBackend` and assert
on the exact characters drawn, so layout can be changed and reviewed without launching anything.

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for how it fits together and which
alternatives were rejected, and [`docs/DESIGN.md`](docs/DESIGN.md) for the game design.

## Status

Playable start to finish: a full run, fights, events, shops, rewards and a boss. The content is
deliberately thin — a handful of spells, enemies and events — because the systems were the goal
and content is cheap to add on top.

Every merge to `main` rebuilds the WebAssembly bundle and republishes it to GitHub Pages.

Not done yet: an animated demo recording, persistent progression between runs, and relics.

## License

MIT — see [LICENSE](LICENSE). Do what you like with it.
