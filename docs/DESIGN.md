# Game Design

A roguelike spell-battler for the terminal. You climb a branching map of encounters, and
every fight is won or lost on how you **combine spells**.

Combination is the whole game. Everything else — the map, the enemies, the events — exists to
give it somewhere to happen.

---

## The core idea

You have five spell slots. Each spell is cheap and unremarkable on its own. On your turn you
don't cast a spell; you **build an incantation** out of several of them, and the result is a
single new spell that is usually not the sum of its parts.

```
Turn 3                                  Mana ●●●●●○○  5/7

  Build:   [ Ember ] + [ Ember ]
  ──────────────────────────────────────────────────────
  Casting: FIREBALL
           6 damage · Flame · single target · Burned (2)

  1 Ember 1MP    2 Frost 2MP    3 Gust 1MP
  4 Surge 3MP    5 Douse 1MP

  [Enter] cast    [Bksp] undo    [Esc] clear
```

Add one more component and it becomes something else entirely:

```
  Build:   [ Ember ] + [ Ember ] + [ Gust ]
  ──────────────────────────────────────────────────────
  Casting: FIRESTORM
           5 damage · Flame · ALL enemies · Burned (2) to all
```

Adding Gust *reduced* the damage and converted the spell to hit everything. That tradeoff is
the point. Building is not "stack more for bigger" — it is a choice about what kind of spell
you want this turn.

---

## Turn structure

- Mana **refills to full at the start of every turn.** It is a per-turn budget, not a
  run-long resource, so each turn is its own self-contained puzzle rather than a question of
  whether you can afford to act at all.
- You may build and cast **as many incantations per turn as mana allows.**
- Each component spent adds its mana cost to the incantation's total.
- Casting resolves the incantation, spends the mana, and clears the build.
- End your turn manually, or when you can no longer afford your cheapest spell.
- Enemies then act, statuses tick, and the next turn begins.

### One subtlety worth understanding

Order does not matter *inside* an incantation — the components are a bag, not a sequence. But
because you can cast more than once per turn, sequencing still matters at the level above:

| What you do | Result |
|---|---|
| Cast `{Douse, Ember}` as **one** incantation | They fuse → **Steam Burst** |
| Cast `Douse`, then `Ember` as **two** incantations | Target is soaked, then burned |

Same two spells, same mana, different outcome. You choose whether to fuse or to sequence.
This is why order-independence inside the bag doesn't flatten the decision space — it moves
the interesting choice up one level, where it's much easier to reason about.

---

## The incantation resolver

Resolution is a pure function of an unordered multiset of components. Same bag in, same spell
out, always — which makes it exhaustively testable and lets the UI recompute the preview on
every keystroke for free.

### Algorithm

1. **Tally** components by element, summing their power.
   `{Ember, Ember, Gust}` → `{Flame: 2 components / 6 power, Gust: 1 / 2}`
2. **Apply fusions.** For each pair of distinct elements present with a `Fuse` rule, replace
   both with the fused element, combining their power.
3. **Choose the base element.** After fusion, the element with the greatest total power. Ties
   are broken by whichever element appears as the base side of a `Modify` rule between them;
   if still tied, by declaration order in the `Element` enum. Deterministic in all cases.
4. **Apply modifiers.** Every remaining non-base element applies its `Modify` rule against the
   base, adjusting power, targeting, pierce, or added effects.
5. **Compute tier** — the number of base-element components. Tier drives both the name and any
   inherent status the element applies.
6. **Name it** from the signature `(element, tier, targeting)`, falling back to a generated
   name when a combination has no authored one.

### The rule table

Only two kinds of entry are needed, and neither depends on the order the player pressed keys.
Order in the *table* expresses the relationship — which element is the base and which reshapes
it — not the order of play.

```ron
// Fusion: two elements become a third.
(Flame, Water) => Fuse(Steam),
(Water, Ice)   => Fuse(Frost),

// Modifier: the first is the base, the second reshapes it.
(Flame, Gust)  => Modify(targeting: All,    power: 0.8),
(Shock, Water) => Modify(power: 2.0),          // water conducts
(Ice,   Earth) => Modify(pierce: true, power: 1.2),
```

Fusion also resolves the awkward tie case naturally: `{Ember, Douse}` is one Flame against one
Water with no dominant element, so rather than needing an arbitrary tiebreak, the pair simply
fuses into Steam.

### Why this shape

The alternative was a recipe table mapping specific spell combinations to specific results.
That gives total authorial control but its content cost grows quadratically — adding a sixth
spell would mean authoring its interaction with the other five, and a twelfth would mean
eleven more.

With rules keyed on *elements* rather than spells, a new spell inherits every interaction its
element already has, for free. That is what makes "I'll add more spells later" cheap instead
of expensive.

A small **override table** sits on top for hand-authored exceptions, so when a particular
combination deserves a bespoke result you can name it without fighting the system.

### Worked examples

Using a seed spell set of Ember (1 MP, Flame, power 3), Douse (1 MP, Water, 2), Gust (1 MP,
Gust, 2), Frost (2 MP, Ice, 4), Surge (3 MP, Shock, 5):

| Build | Resolves to | Why |
|---|---|---|
| `{Ember}` | **Ember** — 3 dmg, Flame, single | Tier 1, no interactions |
| `{Ember, Ember}` | **Fireball** — 6 dmg, single, Burned (2) | Tier 2 stacking |
| `{Ember ×3}` | **Inferno** — 9 dmg, single, Burned (3) | Tier 3 |
| `{Ember, Ember, Gust}` | **Firestorm** — 5 dmg, **all**, Burned (2) | Gust modifies targeting, ×0.8 power |
| `{Ember, Douse}` | **Steam Burst** — 5 dmg, single, Blind (2) | Tie → fuses to Steam |
| `{Surge, Douse}` | **Chain Lightning** — 14 dmg, single | Water conducts, ×2.0 |

---

## Statuses

Statuses are deliberately simple, because the depth in this game comes from composition rather
than from status bookkeeping.

| Status | Effect |
|---|---|
| Burned | Damage at end of round, for N rounds |
| Wet | Incoming Shock damage doubled |
| Frozen | Skips its next turn |
| Blind | Attacks may miss |
| Poisoned | Damage at end of round, ignores shields |

---

## The run

A run is a climb through a randomly generated branching map.

```
        ●           ●              row 5   boss
       / \         /
      ●   ●───────●               row 4
      |    \     /
      ●     ●───●                 row 3
       \   /     \
        ● ●       ●               row 2
         \|      /
          ●─────●                 row 1   start
```

- **~12 nodes across 5 rows**, with 2–3 viable paths and a boss at the top. A run takes
  10–15 minutes: long enough for branching to be a real decision, short enough to balance
  and to demo.
- You see the whole map and choose your next node from those connected to your current one.
- HP and mana capacity carry between nodes. Mana still refills each combat turn.

### Node types

| Node | What happens |
|---|---|
| Fight | A generated encounter. Win to advance. |
| Event | A situation with choices, each with an outcome. |
| Boss | The final fight of the run. |

The framework is built so new node types (shop, rest, treasure) are additive — a new
`NodeKind` variant and a screen to render it.

### Generated fights

Encounters are assembled from a **weighted enemy pool** rather than hand-placed. Each enemy
carries a difficulty weight, and each map row has a difficulty budget; the generator draws
enemies until the budget is spent. Deeper rows get larger budgets.

This means adding an enemy later is a single data entry — it enters the pool and starts
appearing at the depth its weight implies, with no encounter tables to update.

### Generated events

Events are drawn from a **weighted event pool**, with good and bad outcomes both represented.
An event is a prompt, a set of choices, and an outcome per choice:

```ron
Event(
    name: "Abandoned Shrine",
    prompt: "A cracked altar hums faintly. Something is still in there.",
    choices: [
        (text: "Reach inside", outcome: Random([Gain(Spell), Damage(4)])),
        (text: "Leave it",     outcome: Nothing),
    ],
)
```

Outcomes compose from a small vocabulary — gain a spell, lose HP, heal, raise max mana, gain
a spell slot — so new events are pure data with no new code.

---

## Content

All content is data, loaded from `res/` and compiled into the binary so it runs from any
directory.

| File | Holds |
|---|---|
| `spells.ron` | Component spells: cost, element, power |
| `rules.ron` | Fusion and modifier rules; the naming table |
| `enemies.ron` | Enemy stats, behaviour, difficulty weight |
| `events.ron` | Event prompts, choices, outcomes |

The **initial content is deliberately thin** — roughly five spells, four enemies, four events.
Enough to prove every system works end to end and to give something playable. The systems are
the deliverable; the content is meant to grow afterwards.

---

## Deliberately deferred

Named so they're recognised as choices rather than oversights:

- **Persistent progression between runs.** Unlocks and meta-currency are a whole second
  economy; the run itself has to be good first.
- **Relics / passive items.** They interact with everything, which is exactly why they should
  wait until the combination system is settled.
- **Shops and rest nodes.** The `NodeKind` enum is built to take them.
- **Deep enemy AI.** Enemies pick from a small intent set. Enemies that respond to your
  incantations would be excellent later.
- **Balance.** With this little content, balance is not yet a meaningful question.
