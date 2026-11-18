# Compact era gauges

Status: approved by the maintainer on 2026-11-18. Nothing here is built.

Words: era HUD, era gauge and gauge art are used as `CONTEXT.md` defines them.

## Intent

Players have told the maintainer they lose sight of crouching kicks and sweeps in the corner, because the era gauges sit there. Some players like the gauges where they are. So the position becomes a choice: today's layout, or a compact one tucked into the bottom corners under the fighters' feet.

## What exists today

- The era gauges are drawn in the bottom corners by `crates/sf-video/src/era_hud.rs`: 8 pixels in from the side, 8 up from the bottom, up to two rows (8 and 6 pixels tall with a 2-pixel gap), a bar 96 pixels long, and a label after it. With its label a corner is about 146 pixels wide and covers rows 200 to 216 of the 224-row picture.
- A standing fighter's feet land at about row 212 (measured from one screenshot). The gauges therefore sit across the band where a cornered fighter's legs and low attacks are drawn.
- `GAUGE ART` (Ruled, Blocks, Plate) is already a setting, on the Video tab under DISPLAY and in the pause box. Training's frame meter already lifts the gauges above itself, and Dramatic Battle's ally has a compact gauge stacked above the left corner.

## Goals

1. **A setting for the gauge layout** with two values: today's, and compact.
2. **Compact gauges clear the feet.** They are the same gauges, shorter, without labels, flush to the side and bottom edges, and they sit below the foot line.
3. **Compact gauges animate exactly as the full-size ones do** (the maintainer's requirement): the fill sweeping when meter is gained, the flash on a spend, the lit state when a move is affordable, flashing, draining timers, and SF6's greyed Burnout.
4. **Every gauge art and every era's gauge works in both layouts.**

## Non-goals

- Moving the gauges to the top of the screen or to a side rail, a single thin strip along the bottom edge, pips in place of bars, and gauges that fade or move when a fighter stands behind them. All were considered and set aside (see Approaches and the mock-ups).
- A separate choice per player.
- Any change to what the gauges measure.

## Anti-goals

- **Two drawings that drift apart.** If the compact gauge has its own copy of the animations, the first change to one will leave the other behind.
- **A compact gauge that still covers the feet.** If a sweep in the corner is still hidden, the option has failed whatever it looks like.

## Decisions made with the maintainer

- The compact layout is mock-up **F**: both rows at a little over half length, no labels, flush in the corner (`docs/media/kessen-preview/gauge-compact-mockups.png`, a hand-drawn paste-up).
- **Colour is enough** to tell compact gauges apart. They carry no labels, tags or letters. Burnout is shown by its grey.
- The animations are kept in full.
- The approach is one drawing with two layouts (below).

## Claude's calls (correct any of these)

- **The setting is `GAUGE LAYOUT`, with values `CLASSIC` and `COMPACT`,** directly under `GAUGE ART` on the Video tab and in the pause box. `CLASSIC` is the default, so nothing changes for anyone who does not ask.
- **One setting for the whole screen.**
- **All three gauge arts get a compact drawing.** Where an art's detail (a rule, a bevel, Plate's frame) does not fit in fewer rows, it is simplified for compact only.
- **Level numbers stay.** Alpha's and SF3's level digit keeps its place beside the bar; only the bar is shortened.
- **Training's frame meter still takes the bottom rows.** With it on, compact gauges sit directly above it, as the full ones do.
- **The Dramatic Battle ally's gauge** shrinks the same way and stacks above the left corner.
- **Call-outs and the combo counter do not move.**
- **Starting sizes:** a bar 54 pixels long, rows 4 and 3 pixels tall with a 1-pixel gap, flush to the edges. These are from the paste-up and are starting values.

## Approaches

1. **One drawing, two layouts (chosen).** The corner's measurements (bar length, row heights, the gap to the edges, whether a label is drawn) become a layout the drawing is given. Every animation, gauge art and era colour carries over because there is one code path. The cost is care in fitting each gauge art's details into fewer rows.
2. **Draw the full gauge off-screen and shrink it.** Very little code, but 96 pixels to 54 is not a whole-number step, so rules would vanish or double and cells would come out uneven.
3. **A separate compact gauge.** Full control over its look, but the animations would exist twice and drift apart.

The maintainer chose the first on 2026-11-18.

## To verify before trusting the sizes

- The foot line was measured from one standing fighter in the middle of the screen. Before the sizes are fixed, capture real frames of a cornered fighter doing a sweep and a crouching kick, for the tallest and the lowest-reaching fighters, and confirm the compact gauge clears them. If it does not at 8 rows tall, the gauge gets shorter before anything else changes.
- The real gauges have rules and shadows the paste-up left out. The true compact size may be a row or two taller than drawn.

## Left to the builder

- How the layout is passed through the drawing code and how each gauge art simplifies for compact.
- The setting's storage key and how the pause box cycles it.
- Tests at the seams the era HUD's existing tests use: the drawn frame for each gauge art and layout, and the setting through the settings screen and the pause box.

## Words to add to `CONTEXT.md`

**Gauge layout**: where and how large the era gauges are drawn, one of Classic or Compact.
