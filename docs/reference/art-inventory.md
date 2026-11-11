# Art inventory: the twelve tournament fighters

This document maps the animation art the 1987 ROM already holds for each
tournament fighter, set against what a complete SF1+ moveset needs. It is
the starting map for the art pipeline that will generate missing frames
from the ROM's own sprites. Written 2026-11-02.

The ROM is the reference version's program region (`maincpu`, read as data
only) and its sprite region (`gfx3`). Nothing here was changed in
`crates/`.

## How to read this

**Tags.** Every claim carries one of three tags.

- **M** (measured): read from the ROM by the probe described below.
- **V** (viewed): the probe drew the poses, and the description is my
  reading of the pictures. It is a judgement, so treat it as inferred.
- **I** (inferred): taken from a reference doc or a code comment, not
  checked against the ROM here.

**Words.**

- A **stream** is a move's animation list in the program ROM. Addresses
  are program addresses (`0x0c9b6`).
- A **frame** is one 0x20-byte frame record. A table move also counts its
  0x30-byte header, which holds the first pose. A frame can be held for
  several ticks.
- A **pose** is what a frame draws: the top-left sprite code, the sprite
  size, and the grid. `0x0224 B3x3` is code 0x224 drawn as a grid of 3 rows
  by 3 columns of big (32 px) sprites. `s` marks small (16 px) sprites.
  Two frames with the same key draw the same picture.
- **Poses** in a count include the stance or jump poses a stream passes
  through. **New** poses are the ones not already in the fighter's
  stance, walk, crouch, block and jump moves.
- **Ryu parity** is the art coverage Ryu has (see the Ryu section). The
  "missing" columns count against it unless they say otherwise.

## Method

The probe is a throwaway Rust binary in a scratch crate outside the
repository, with path dependencies on `sf-engine` and `sf-video`. It loads
the ROM with `sf_video::romset::load_assets` and reads `maincpu` as data.
Its source is not committed.

1. **Move tables.** For each fighter it reads every 0x30-byte header of
   the table that `TournamentFighter::move_table` names. The spacing
   between tables is 0x5d0 for Ryu and Ken (31 moves, 0x00..0x1e) and
   0x270 for every boss (13 moves, 0x00..0x0c). Sagat's table ends at
   0x1845c. **M**
2. **Stream walk.** It walks each stream the way `fighter.rs` `animate`
   and `load_frame` do. A byte other than 0xfd..0xff starts a 0x20-byte
   frame record (hold, duration, code, attr, dy, dx, grid, boxes, flags,
   x-offset, hit function). 0xff is a 6-byte routine call, 0xfe a goto,
   0xfd the end. It follows every branch once: the goto after the CPU
   follow-up call (0x45630, which can resume 8 bytes on), and all three
   frame groups of Geki's teleport choice (0x45a34). A move whose state
   has bit 7 clear is never animated, so it is its header pose alone.
   There is no frame-record decoder in the crates to reuse, so the probe
   has its own; it mirrors `load_frame` byte for byte. **M**
3. **Jump attacks and recoils (Ryu, Ken).** The apex tables (Ryu 0x45386 /
   0x4539e, Ken 0x4545c / 0x45474) give the jump-attack streams. The
   blocked-kick recoil streams come from `fighter::hit_function`. **M**
4. **Boss attacks.** For each boss the probe reads the opponent pointer
   arrays at 0x19908 (country 0) and walks every 8-byte record of its
   airborne and ground reaction tables and its three attack-record
   tables, over all eight difficulty and variant rows (ai-tables.md). That
   gives every stream the CPU can start, with its state and move bytes
   and how often the records name it. It also checks each stream that
   `roster.rs` binds to a human boss's buttons against those records.
   **M**
5. **Reactions.** It walks the hit (0x30..0x3c) and knock-out
   (0x40..0x4a) reaction tables: Ryu 0x44662 / 0x446ca and Ken 0x4473a /
   0x4479c (from `fighter.rs` `react`), and each boss's own pair from the
   pointer arrays (0x1a128 / 0x1a1c8 for country 0). Each table has 13
   hit entries and 11 knock-out entries; the bytes after them are code.
   **M**
6. **Sharing.** It collects every 16 px tile each fighter's poses draw and
   compares the sets between fighters. **M**
7. **Pictures.** It decodes the poses from `gfx3` (`sf_video::decode`,
   pen index as grey) into contact sheets, one row per stream. I looked at
   them to tell punches from kicks and standing from crouching. **V**
8. **Leftovers.** It scans the program ROM for frame records that carry a
   known hit function but that no walk reached, and it counts the
   non-blank `gfx3` tiles that no walked pose draws. **M**

**Limits.**

- Effects were not walked: projectiles, sparks, Geki's smoke, the bonus
  stages and the ending. Their art is outside these counts.
- A pose is keyed by its code, so two codes with identical pixels count
  twice.
- Frame counts include the stance frame most streams end on.

## What is per fighter and what is shared

| Finding | Tag |
| --- | --- |
| Every fighter's poses use tiles no other fighter's walked poses use. Across all twelve, no 16 px tile and no pose is shared. | M |
| Ken is not a palette swap of Ryu's tiles. His poses are a separate copy, mostly at Ryu's code + 0x800 (Ryu's 0x06xx poses, the walk, win and one Tatsumaki pose, at + 0x100). Of 1,210 compared tiles, 321 are pixel-identical and 585 have the same silhouette; 16% of pixels differ. | M |
| Hit reactions are per fighter: each fighter has its own 13 hit streams and 11 knock-out streams, drawn with its own tiles. | M |
| In Ryu's tables the entries split by state: 0x30..0x32 standing hit (state 0x8a), 0x33..0x39 knocked back or hit in the air (0x89), 0x3a..0x3c crouching hit (0x8c); knock-outs 0x40 and 0x48..0x4a (0x8c) and 0x41..0x47 (0x89). The bosses' tables mostly follow this; Mike's 0x3a and 0x40, for example, do not. | M (states), I (names: punch.md, crouch-attacks.md, jump-attacks.md) |
| The damage entry picks the column by the defender's posture: standing, crouching (+9 & 7 = 4) or airborne (1). | I (punch.md, crouch-attacks.md) |
| A real crouching-hit pose exists only for Ryu, Ken, Retsu and Geki. The other eight fill entries 0x3a..0x3c with standing-hit or knock-back poses. | M |
| Every knock-out stream ends on a lying pose. No hit stream gets up from lying, so there is no get-up art anywhere. (One oddity: Adon's hit entry 0x37 ends on his lying pose `0x32e8`.) | M |
| The win pose is table move 0x0c for every fighter. Geki's move 0x0c points at the same stream as his teleport, move 4 (0x0b6a8): he wins by vanishing. | M (stream addresses), I (move 0x0c is the victory pose: round-result.md) |
| The intro is table move 0: Ryu and Ken walk in; the bosses jump in, except Mike, who walks, and Sagat, who hops on his knee poses. | M (streams), V |
| There is no lose pose. A knocked-out fighter lies on the floor; after a time-over the loser does nothing special. | M (knock-out streams), I (round-outcomes.md) |
| No fighter has a throw. None of the 31 Ryu/Ken moves, the boss tables or the AI records is a throw, and no reference doc mentions one. | M (no such stream), I (docs) |
| The VS-screen busts are per fighter but live in the background layer (`gfx1`), not with the sprites. | I (`sf-video` `busts.rs`) |

## Ryu: the reference

Move table 0x1605c. Ryu and Ken are the only fighters a player controls in
1987 (the bosses are CPU-only: `roster.rs`, **I**), so their holdings define
Ryu parity.

| Need | In the ROM: stream, frames, poses | Missing for SF1+ | Tag |
| --- | --- | --- | --- |
| Stance | move 1, header pose `0x0063 B3x2` | — | M |
| Walk forward / back | move 2 0x00502 and move 3 0x005b0: 6 frames, 4 poses each (the same 3 walk poses) | — | M |
| Crouch | move 5 `0x0120`; also crouch-forward move 6 `0x0126` and crouch-back move 7 `0x0163` | — | M |
| Jumps | neutral move 9 0x0065e: 10 frames, 4 poses (a tuck); forward 0xa 0x0078c and back 0xb 0x0089a: 9 frames, 6 poses (a somersault). Two jump arcs. | — | M, V |
| Standing high block | move 4, header pose `0x0288 B3x2` | — | M |
| Crouching low block | move 8, header pose `0x0123` | — | M |
| Standing normals | LP 0x0d 0x009ca (3 fr, 3 poses, 2 new); MP 0x0f 0x00a6e (5, 4, 3 new); HP 0x10 0x00af0 (6, 5): MP's poses plus the block pose; LK 0x14 0x00e00 (4, 3, 2 new); MK 0x15 0x00e62 (8, 7, 6 new); HK 0x16 0x00fac (8, 7): five of MK's poses plus a wind-up `0x0426`, with the same strike pose `0x02e6`. **4 distinct animations: LP, MP=HP, LK, MK=HK.** | — | M |
| Crouching normals | punches 0x11/0x12/0x13 (light) and 0x1c/0x1d/0x1e (medium, heavy), streams 0x00b92, 0x00c14, 0x00c56, 0x00d38, 0x00d18: all six strike with the same extended pose `0x01eb s4x5`; light and medium kick 0x17 0x01116 (3 fr, 2 poses); sweep 0x18 0x01158 (6 fr, 6 poses). **3 distinct animations.** | — | M |
| Jumping normals | neutral punch 0x20 0x01584 (all strengths); diagonal punch 0x21/0x22 0x0166c, 0x01734 (same poses); neutral light/medium kick 0x23 0x017fc; neutral heavy kick 0x26 0x01904 (the Tatsumaki spin pose); diagonal kick 0x24/0x25 0x019f2, 0x01aba (same poses). Each 6–8 frames, 2–4 new poses. **5 distinct animations.** | — | M |
| Ground throw | none | throw + thrown reaction | M, I |
| Specials | Shoryuken 0x19 0x01200 (12 fr, 6 poses, 5 new); Hadoken 0x1a 0x01362 (5 fr, 4 poses, 3 new; projectile template 0x186dc, code `0x01e8`); Tatsumaki 0x1b 0x013ea (13 fr, 8 poses, 4 new; five spins by a loop) | — | M, I (names: specials.md, hadoken.md) |
| Hit reactions | 13 streams 0x01b82..0x02124; 6 poses not in his movement set: `0x00c0`, `0x00c4`, `0x0420`, `0x0540`, `0x0480` (fall), `0x0548` (crouching hit) | get-up, if SF1+ knocks down mid-round | M |
| Knockdown | 11 knock-out streams 0x02166..0x02496; fall `0x0480 B2x2`, lying `0x0484 B1x4` | as above | M |
| Win | move 0xc 0x009a8: 2 frames, 2 poses | — | M |
| Lose | none (knock-out lying pose only) | lose pose, if SF1+ wants one | M, I |
| Extra | blocked-kick recoils 0x00f4a, 0x010b4, 0x01094 (3–4 frames, poses reused); move 0x0e 0x00a0c, a light punch with a different wind-up that input cannot reach (standing-attacks.md); intro move 0 0x00400 (9 frames, 7 poses) | — | M, I |

Ryu holds **71 distinct poses** over every walked stream. **M**

**Ryu parity, as a checklist.** Stance, walk, crouch, two jump arcs,
standing and crouching block, 4 standing normal animations (light punch,
medium/heavy punch, light kick, medium/heavy kick), 3 crouching (punch,
light/medium kick, sweep), 5 jumping (neutral punch, diagonal punch,
neutral light/medium kick, neutral heavy kick, diagonal kick), specials,
standing / crouching / airborne hit, knock-out fall and lying pose, win.

## Ken

Move table 0x1662c. Ken's table has the same layout as Ryu's, move for
move. All 55 compared streams (31 table moves, 24 reaction streams) and
his jump-attack and recoil streams have the same frame and pose counts as
Ryu's. Only the codes and a few hold times differ (his medium kick and
Hadoken run a few ticks longer). **M**

| Need | In the ROM | Missing for SF1+ | Tag |
| --- | --- | --- | --- |
| Everything Ryu has | the same 31 moves: walk 0x025e0 / 0x0268e, jumps 0x0273c / 0x0286a / 0x02978, normals 0x02aa8..0x03236, Shoryuken 0x032de, Hadoken 0x03440, Tatsumaki 0x034c8; jump attacks 0x03662, 0x0374a, 0x03812, 0x038da, 0x039e2, 0x03ad0, 0x03b98; recoils 0x03028, 0x03192, 0x03172 | — | M |
| Hit reactions / knockdown | 13 + 11 streams from 0x03c60; poses `0x08c0`, `0x08c4`, `0x0c20`, `0x0c80`, `0x0d40`, `0x0d48`, lying `0x0c84` | get-up (as Ryu) | M |
| Ground throw | none | throw + thrown reaction | M, I |
| Lose | none | as Ryu | M, I |

Ken holds **71 distinct poses**. His tiles are his own (see above). **M**

## Retsu

Move table 0x16e6c. 38 distinct poses. Tiles 0x1000..0x13d7. **M**

| Need | In the ROM | Missing (Ryu parity) | Tag |
| --- | --- | --- | --- |
| Stance | `0x1000 B3x2` | — | M |
| Walk forward / back | 0x0c332, 0x0c3a0: 4 frames, 3 poses each | — | M |
| Crouch | move 5 0x0c40e: 5 frames, 3 poses (`0x124d s4x3`) | — | M |
| Jumps | 0x0c4a8, 0x0c5c8, 0x0c708: 9–10 frames, the same 3 airborne poses for all three | a diagonal arc, if wanted (Ryu has two) | M |
| Standing high block | move 4 0x0c890: 3 frames (`0x1060`, `0x1065`) | — | M |
| Crouching low block | move 8 0x0c900: 6 frames (`0x120d`) | — | M |
| Standing normals | jab 0x0c9b6 (4 fr, 3 poses, 2 new); lunging punch 0x0ca46 (6, 4, 3 new); mid kick 0x0cb16 (6, 4, 3 new); high kick 0x0cbe6 (7, 7, 6 new) | — (4 of 4) | M, V |
| Crouching normals | sweep 0x0cf1a (10 fr, 8 poses, 5 new; starts standing, ends low). Its hit function has no block test. | crouching punch, light/medium kick (2) | M, V, I (cpu-follow-up.md) |
| Jumping normals | flying kick 0x0ccd6 (8 fr, 5 poses, 1 new) and 0x0cdf8 (same poses, higher arc) | 4 of 5 | M, V |
| Ground throw | none | throw + thrown reaction | M, I |
| Specials | none in 1987 | open: the README names none (`sf1plus.rs` names Kiai Shot) | I |
| Hit reactions | 13 streams 0x04f68..0x0555a; 4 new poses incl. a crouching hit `0x12ec` | get-up | M |
| Knockdown | 11 streams 0x0559c..0x058cc; fall `0x1209`, lying `0x11e9 s2x7` | — | M |
| Win | move 0xc 0x0c828: 3 frames, 3 poses (`0x1380`, `0x1384`) | — | M |
| Lose | none | lose pose, if wanted | M, I |

`roster.rs` binds LP 0x0c9b6, MP 0x0ca46, HP 0x0cb16 (the mid kick), LK
0x0cbe6, MK 0x0ccd6 (the jump kick), HK 0x0cf1a. All are AI-record
streams. **M**

## Geki

Move table 0x16bfc. 32 distinct poses. Tiles 0x0667..0x137f: his jump
poses sit inside the span that holds Ryu's tiles, without sharing any.
**M**

| Need | In the ROM | Missing (Ryu parity) | Tag |
| --- | --- | --- | --- |
| Stance | `0x0e00 B3x2` | — | M |
| Walk forward / back | 0x0afd2, 0x0b040: 4 frames, 3 poses each | — | M |
| Crouch | move 5 0x0b0ae: 5 frames (`0x0fc7 s4x3`) | — | M |
| Jumps | neutral 0x0b148 (11 fr, 4 poses); forward / back 0x0b2a8, 0x0b3e8 (10 fr, 6 poses, a somersault). Two arcs, like Ryu. | — | M, V |
| Standing high block | none. Move 4 is the teleport (0x0b6a8, 13 frames, 6 poses incl. smoke). No AI record has a block state. | standing block | M |
| Crouching low block | none. Move 8 is a second teleport stream (0x0b7c6). | crouching block | M |
| Standing normals | claw thrust 0x0b968 (6 fr, 4 poses, 2 new); front kick 0x0bb54 (6, 4, 2 new) | a second punch and a second kick (2) | M, V |
| Crouching normals | crouching claw 0x0ba38 (8 fr, 5 poses, 2 new). A smoke variant 0x0c0f4 (8, 5, 2 new) calls the smoke routine and has no active frame. | light/medium kick, sweep (2) | M, V |
| Jumping normals | flip into a diving claw 0x0bc24 (8 fr, 8 poses, 2 new); a short hop into the same claw 0x0be5c (9, 6). An unreferenced mirror of 0x0bc24 sits at 0x0bd40 with the same poses. | 4 of 5 | M, V |
| Ground throw | none | throw + thrown reaction | M, I |
| Specials | 1987 has two: the shuriken thrown from a hop 0x0bf98 (10 fr, 6 poses, 2 new; templates 0x1849c / 0x184bc / 0x184dc, code `0x0e6e`) and the teleport (moves 4 and 8). | open: the README names none (`sf1plus.rs`: Shuriken, Teleport Strike) | M, I (geki.md) |
| Hit reactions | 13 streams from 0x045bc; 4 new poses incl. a crouching hit `0x0f86` | get-up | M |
| Knockdown | 11 streams from 0x04bf0; fall `0x0eac`, lying `0x0f66 B1x3` | — | M |
| Win | move 0xc is the teleport stream 0x0b6a8 | a win pose, if vanishing will not do | M |
| Lose | none | lose pose, if wanted | M, I |

`roster.rs` binds LP 0x0bf98 (the shuriken), MP 0x0bb54, HP 0x0ba38, LK
0x0bc24, MK and HK 0x0be5c, down+LP 0x0b968, and back+punch the teleport
move 4. **M**

## Joe

Move table 0x1734c. 37 distinct poses. Tiles 0x1700..0x1adb. **M**

| Need | In the ROM | Missing (Ryu parity) | Tag |
| --- | --- | --- | --- |
| Stance | `0x1700 B3x2` | — | M |
| Walk forward / back | 0x0dd2e, 0x0de0e: 7 frames, 4 poses each | — | M |
| Crouch | none. Move 5's stream pointer (0x0e36a) lands 0x1a bytes inside the frame record at 0x0e350, so the slot is broken. Move 8 is empty. | crouch | M |
| Jumps | 0x0df88, 0x0e0a8, 0x0e1e8: 9–10 frames, one arc | a diagonal arc, if wanted | M |
| Standing high block | move 4 0x0e370: 3 frames (`0x176c`) | — | M |
| Crouching low block | none | crouching block | M |
| Standing normals | jab 0x0e3ba (4 fr, 3 poses, 2 new); straight 0x0e44a (6, 4, 3 new); low kick 0x0e7fc (6, 4, 3 new); high kick 0x0e8cc (7, 7, 6 new); low kick then punch 0x0e706 (7, 6, 5 new, two hits) | — (4 of 4, plus one) | M, V |
| Crouching normals | a ducking body punch 0x0e60a (7 fr, 5 poses, 3 new). The stream sets the crouch posture, but its active pose is standing height. An unreferenced variant with the same poses is at 0x0e51a. | light/medium kick, sweep (2) | M, V |
| Jumping normals | flying kick 0x0e9bc (8 fr, 7 poses, 4 new) and 0x0eade (same poses) | 4 of 5 | M, V |
| Ground throw | none | throw + thrown reaction | M, I |
| Specials | none in 1987 | open: the README names none (`sf1plus.rs`: Rolling Sobat, Dynamite Straight) | I |
| Hit reactions | 13 streams from 0x065f8; 4 new poses; no crouching-hit pose | crouching hit, get-up | M |
| Knockdown | 11 streams from 0x06ce4; lying `0x1a64 s2x7` | — | M |
| Win | move 0xc 0x0e308: 3 frames, 2 poses | — | M |
| Lose | none | lose pose, if wanted | M, I |

`roster.rs` binds LP 0x0e3ba, MP 0x0e44a, HP 0x0e60a, LK 0x0e7fc, MK
0x0e8cc, HK 0x0eade. **M**

## Mike

Move table 0x170dc. 23 distinct poses, the fewest after Sagat. Tiles
0x1400..0x16ff. **M**

| Need | In the ROM | Missing (Ryu parity) | Tag |
| --- | --- | --- | --- |
| Stance | `0x1400 s7x3` (small sprites) | — | M |
| Walk forward / back | 0x0d1f8, 0x0d42c: 7 frames, 3 poses each | — | M |
| Crouch | move 5 0x0d660: 4 frames (`0x156a B2x2`) | — | M |
| Jumps | 0x0d6d4, 0x0d2d2, 0x0d506: 11 frames, one arc (tuck `0x1608`) | a diagonal arc, if wanted | M |
| Standing high block | move 4 0x0d87c: one guard pose `0x1648`. The AI uses it with both the standing (0xa2) and crouching (0xa4) block states. | — | M |
| Crouching low block | none of its own (move 8 is empty) | a second guard pose | M |
| Standing normals | jab 0x0d8c6 (4 fr, 3 poses, 2 new); straight 0x0d956 (6, 4, 3 new); lunging hook 0x0db1c (7, 6, 5 new). No kicks. | two kick animations (2) | M, V |
| Crouching normals | ducking low punch 0x0da26 (7 fr, 4 poses, 2 new; state 0xc4) | light/medium kick, sweep (2) | M, V |
| Jumping normals | none | 5 of 5 | M |
| Ground throw | none | throw + thrown reaction | M, I |
| Specials | none in 1987 | open: the README names none (`sf1plus.rs`: Bison Rush, Iron Upper) | I |
| Hit reactions | 13 streams from 0x05914; 4 new poses; no crouching-hit pose | crouching hit, get-up | M |
| Knockdown | 11 streams from 0x061a0; lying `0x15e8 s2x8` | — | M |
| Win | move 0xc 0x0d814: 3 frames, 3 poses | — | M |
| Lose | none | lose pose, if wanted | M, I |

`roster.rs` binds LP and MK 0x0d8c6, MP 0x0d956, HP and HK 0x0db1c, LK
0x0da26: Mike's kick buttons start punches. The README says "Mike's SF1+
kicks stay on LK, MK, and HK". **M**, **I**

## Lee

Move table 0x175bc. 34 distinct poses. Tiles 0x1c00..0x1ffb (his lying pose
is at 0x1388). **M**

| Need | In the ROM | Missing (Ryu parity) | Tag |
| --- | --- | --- | --- |
| Stance | `0x1c00 B3x3` | — | M |
| Walk forward / back | 0x0ed54, 0x0edc2: 4 frames, 3 poses each | — | M |
| Crouch | none (moves 5 and 8 empty) | crouch | M |
| Jumps | 0x0ee30, 0x0efb0, 0x0f0f0: 10–12 frames, one arc | a diagonal arc, if wanted | M |
| Standing high block | move 4 0x0f256: 3 frames (`0x1f40`) | — | M |
| Crouching low block | none | crouching block | M |
| Standing normals | palm 0x0f366 (3 fr, 3 poses, 1 new); punch 0x0f3d6 (6, 4, 2 new); lunging punch 0x0f4a6 (8, 5, 3 new, a small hop); side kick 0x0f5c8 (5, 4, 3 new); high kick 0x0f678 (8, 7, 5 new) | — (4 of 4, plus one) | M, V |
| Crouching normals | low sweep 0x0fc8e (7 fr, 6 poses, 4 new) | crouching punch, light/medium kick (2) | M, V |
| Jumping normals | flying kick 0x0f788 and 0x0f92a (12–13 fr, 7 poses, 2 new, two hits); a second flying kick 0x0faec (12, 6, 2 new) | 3 of 5 | M, V |
| Ground throw | none | throw + thrown reaction | M, I |
| Specials | the README names Zesshouohken and Double Kick. No stream is documented as either. | both, unless an existing stream stands in | M, I |
| Hit reactions | 13 streams from 0x0705c; 5 new poses; no crouching-hit pose | crouching hit, get-up | M |
| Knockdown | 11 streams from 0x077a2; lying `0x1388 s2x7` | — | M |
| Win | move 0xc 0x0f284: 8 frames, 5 poses | — | M |
| Lose | none | lose pose, if wanted | M, I |

`roster.rs` binds LP 0x0f366, MP 0x0f3d6, HP 0x0f4a6, LK 0x0f678, MK
0x0f5c8, HK 0x0faec. **M**

## Gen

Move table 0x1782c. 33 distinct poses. Tiles 0x2000..0x239b. **M**

| Need | In the ROM | Missing (Ryu parity) | Tag |
| --- | --- | --- | --- |
| Stance | `0x2000 B3x2` | — | M |
| Walk forward / back | 0x0febe, 0x0ff2c: 4 frames, 3 poses each | — | M |
| Crouch | none | crouch | M |
| Jumps | 0x0ff9a, 0x100fa, 0x1021a: 9–11 frames, one arc | a diagonal arc, if wanted | M |
| Standing high block | move 4 0x10340: 3 frames (`0x200c`) | — | M |
| Crouching low block | none | crouching block | M |
| Standing normals | jab 0x103b0 (4 fr, 3 poses, 2 new; its frame with a hit function lacks the active bit, so it never hits); punch 0x10440 (5, 4, 3 new); front kick 0x104f0 (4, 3, 2 new); an arm flurry 0x10b4e (12, 7, 6 new, three active frames) | a second kick (1) | M, V |
| Crouching normals | a rolling low kick 0x10a26 (8 fr, 5 poses, 4 new; crouch posture) | crouching punch and one more kick (2) | M, V |
| Jumping normals | flying kick 0x10580 and 0x10702 (11 fr, 6 poses, 2 new); a second flying kick 0x10884 (12, 6, 2 new) | 3 of 5 | M, V |
| Ground throw | none | throw + thrown reaction | M, I |
| Specials | the README names Hyakurenko and Gekiro, with no stance switch. No stream is documented as either; the flurry 0x10b4e is the nearest existing art. | both, unless an existing stream stands in | M, V, I |
| Hit reactions | 13 streams from 0x07b7a; 3 new poses; no crouching-hit pose | crouching hit, get-up | M |
| Knockdown | 11 streams from 0x08260; lying `0x2348 s2x7` | — | M |
| Win | move 0xc 0x1036e: 3 frames, 2 poses | — | M |
| Lose | none | lose pose, if wanted | M, I |

`roster.rs` binds LP 0x103b0, MP 0x10440, HP 0x104f0, LK 0x10580, MK
0x10a26, HK 0x10884. It does not bind 0x10702 or 0x10b4e. **M**

## Birdie

Move table 0x17a9c. 30 distinct poses. Tiles 0x2400..0x291f. **M**

| Need | In the ROM | Missing (Ryu parity) | Tag |
| --- | --- | --- | --- |
| Stance | `0x2400 B4x2` | — | M |
| Walk forward / back | 0x10e58, 0x10ed2: 4 frames, 3 poses each | — | M |
| Crouch | none | crouch | M |
| Jumps | 0x10f46, 0x110e6, 0x11246: 11–13 frames, one arc | a diagonal arc, if wanted | M |
| Standing high block | move 4 0x113ac: 3 frames (`0x2588`) | — | M |
| Crouching low block | none | crouching block | M |
| Standing normals | jab 0x1141c (2 fr, 2 poses, 1 new); straight 0x1146c (5, 4, 3 new); an overhead hammer 0x1151c (5, 5, 4 new, three active frames); kick 0x115cc (4, 4, 3 new); a lunging headbutt 0x11b42 (8, 5, 3 new, four active frames) | a second kick (1) | M, V |
| Crouching normals | none | 3 of 3 | M, V |
| Jumping normals | a diving attack 0x1165c and 0x119a0 (12 fr, 7 poses, 5 new). The unreferenced 0x117fe has the same poses (birdie.md). | 4 of 5 | M, V, I |
| Ground throw | none | throw + thrown reaction | M, I |
| Specials | the README names Bull Head and Bull Horn. No stream is documented as either; the headbutt 0x11b42 is the nearest existing art. | both, unless an existing stream stands in | M, V, I |
| Hit reactions | 13 streams from 0x085d8; 3 new poses; no crouching-hit pose | crouching hit, get-up | M |
| Knockdown | 11 streams from 0x08c9e; lying `0x2680 B1x5` | — | M |
| Win | move 0xc 0x113da: 3 frames, 2 poses | — | M |
| Lose | none | lose pose, if wanted | M, I |

`roster.rs` binds LP 0x1141c, MP 0x1146c, HP 0x1151c, LK 0x115cc, MK
0x1165c, HK 0x119a0. It does not bind 0x11b42. **M**

## Eagle

Move table 0x17d0c. 24 distinct poses. Tiles 0x23a8..0x2bb7. **M**

| Need | In the ROM | Missing (Ryu parity) | Tag |
| --- | --- | --- | --- |
| Stance | `0x2920 B3x2` | — | M |
| Walk forward / back | 0x11dd8, 0x11e46: 4 frames, 3 poses each | — | M |
| Crouch | none | crouch | M |
| Jumps | 0x11eb4, 0x12014, 0x12174: 11 frames, one arc | a diagonal arc, if wanted | M |
| Standing high block | move 4 0x122da: 3 frames (`0x2aa0`) | — | M |
| Crouching low block | none as a crouch. Move 8 0x1232e holds a second guard pose `0x2aa4`, but with the standing-block state 0xa2. | crouching block, unless `0x2aa4` serves | M |
| Standing normals | stick jab 0x1239e (3 fr, 3 poses, 2 new); spinning sticks 0x12408 (6, 6, 5 new, four active frames); the same art run longer 0x124d8 (10 frames, eight active). No kicks. | two kick animations (2) | M, V |
| Crouching normals | a low lunging stick thrust 0x12628 (6 fr, 4 poses, 3 new; its hit function 0x47ba8 tests the crouching block 0x24) | two crouching kicks (2) | M, V |
| Jumping normals | a jumping stick strike, 0x126f8, 0x1285a and 0x129bc (10 fr, 5 poses, 1 new; one art, three arcs) | 4 of 5 | M, V |
| Ground throw | none | throw + thrown reaction | M, I |
| Specials | the README names Manchester Black and St. Andrew's Green. No stream is documented as either. `roster.rs` calls 0x124d8 a "repeated-hit barrage" and 0x129bc a "leaping attack". | both, unless an existing stream stands in | M, I |
| Hit reactions | 13 streams from 0x09016; 3 new poses; no crouching-hit pose | crouching hit, get-up | M |
| Knockdown | 11 streams from 0x096fc; lying `0x2b69 s2x7` | — | M |
| Win | move 0xc 0x1235c: 3 frames, 2 poses | — | M |
| Lose | none | lose pose, if wanted | M, I |

`roster.rs` binds LP 0x1239e, MP and HP 0x124d8, LK 0x129bc, MK 0x126f8,
HK 0x1285a. It does not bind 0x12408 or 0x12628. **M**

## Adon

Move table 0x17f7c. 29 distinct poses. Tiles 0x3060..0x33bf. **M**

| Need | In the ROM | Missing (Ryu parity) | Tag |
| --- | --- | --- | --- |
| Stance | `0x3060 B3x2` | — | M |
| Walk forward / back | 0x12cb2, 0x12d4c: 5 frames, 3 poses each | — | M |
| Crouch | none | crouch | M |
| Jumps | 0x12de6, 0x12f66, 0x130e6: 12 frames, one arc | a diagonal arc, if wanted | M |
| Standing high block | move 4 0x1326c: 3 frames (`0x312c`) | — | M |
| Crouching low block | none | crouching block | M |
| Standing normals | jab 0x132dc (4 fr, 3 poses, 2 new, two active frames); body punch 0x1336c (4, 3, 2 new); low kick 0x133fc (4, 3, 2 new); knee then high kick 0x13850 (9 fr, 9 poses, 8 new, two hits) and 0x1398c (18 fr, the same poses, four hits, turning between them). The knee combos have dy 0 on every frame: they step forward, they do not leave the ground. | — (4 of 4) | M, V |
| Crouching normals | none | 3 of 3 | M, V |
| Jumping normals | flying kick 0x1348c and 0x1366e (14 fr, 7 poses, 3 new, two hits) | 4 of 5 | M, V |
| Ground throw | none | throw + thrown reaction | M, I |
| Specials | the README names Jaguar Kick and Rising Jaguar. No stream is documented as either; the flying kick is the nearest existing art. | both, unless an existing stream stands in | M, V, I |
| Hit reactions | 13 streams from 0x09a74; 4 new poses; no crouching-hit pose | crouching hit, get-up | M |
| Knockdown | 11 streams from 0x0a0fa; lying `0x32e8 s2x7` | — | M |
| Win | move 0xc 0x1329a: 3 frames, 2 poses | — | M |
| Lose | none | lose pose, if wanted | M, I |

`roster.rs` binds LP 0x132dc, MP 0x1336c, HP 0x133fc, LK 0x1348c, MK
0x1366e, HK 0x13850. **M**

## Sagat

Move table 0x181ec. 20 distinct poses, the fewest. Tiles 0x2c00..0x2f7d.
**M**

| Need | In the ROM | Missing (Ryu parity) | Tag |
| --- | --- | --- | --- |
| Stance | `0x2c00 B4x2` | — | M |
| Walk forward / back | 0x13d6e, 0x13ddc: 4 frames, 3 poses each | — | M |
| Crouch | none | crouch | M |
| Jumps | none: moves 9, 0xa and 0xb are empty, and no AI record jumps | a jump arc | M |
| Standing high block | move 4 0x13e50: 3 frames (`0x2d8c`) | — | M |
| Crouching low block | none | crouching block | M |
| Standing normals | jab 0x13ec0 (2 fr, 2 poses, 1 new; its hit frame lacks the active bit, so it never hits); lunging punch 0x13f10 (4, 3, 1 new); low kick 0x13fa0 (4, 3, 2 new; its hit function 0x47f86 tests the crouching block 0x24); high kick 0x14030 (5, 4, 3 new); high kick then low kick 0x143a4 (8, 5, 4 new) | — (4 of 4, plus one) | M, V |
| Crouching normals | none | 3 of 3 | M |
| Jumping normals | none | 5 of 5 | M |
| Ground throw | none | throw + thrown reaction | M, I |
| Specials | the README names high and low Tiger Shot and Tiger Knee. Tiger Shot 0x144b4 (5 fr, 3 poses, 2 new) spawns one projectile, template 0x1873c (code `0x3048`); it is the ROM's only call to the spawn routine 0x45ace. Tiger Knee 0x14242 (10 fr, 4 poses, 3 new; a small hop) and 0x140e0 (same poses). | low Tiger Shot | M, I (roster.rs comments name the knee) |
| Hit reactions | 13 streams from 0x0a472; 3 new poses; no crouching-hit pose | crouching hit, get-up | M |
| Knockdown | 11 streams from 0x0ab38; lying `0x2e88 B1x4` | — | M |
| Win | move 0xc 0x13e7e: 3 frames, 2 poses | — | M |
| Lose | none | lose pose, if wanted | M, I |

`roster.rs` binds LP 0x13ec0, MP 0x13f10, HP 0x13fa0, LK 0x14030, MK
0x140e0, HK 0x143a4, down+punch 0x144b4, down+kick 0x14242. **M**

## Summary: missing animations

**This is an estimate.** It counts against Ryu parity and prices each gap
in new poses:

- a normal animation: 2 poses (wind-up and strike; recovery reuses the
  stance);
- a special: 4 poses (effects not counted);
- crouch, crouching block, standing block or crouching hit: 1 pose each;
- a jump arc: 3 poses;
- a throw: 5 poses (3 for the thrower, 2 for the fighter being thrown);
- a get-up: 2 poses.

Ryu's own normals take 2 to 6 new poses each, so the 2-pose rate is low
for kicks. The extra jump arc and the lose pose are not counted.

| Fighter | Normals missing (stand / crouch / jump) | Other missing | Specials | Est. new poses (no specials) | With specials | Normals missing if every button gets its own animation |
| --- | --- | --- | --- | ---: | ---: | ---: |
| Ryu | 0 / 0 / 0 | throw, get-up | 0 | 7 | 7 | 6 |
| Ken | 0 / 0 / 0 | throw, get-up | 0 | 7 | 7 | 6 |
| Retsu | 0 / 2 / 4 | throw, get-up | open | 19 | 19 + 4 per special | 12 |
| Geki | 2 / 2 / 4 | both blocks, throw, get-up | open (shuriken and teleport exist) | 25 | 25 + 4 per new special | 14 |
| Joe | 0 / 2 / 4 | crouch, crouching block, crouching hit, throw, get-up | open | 22 | 22 + 4 per special | 11 |
| Mike | 2 / 2 / 5 | crouching block, crouching hit, throw, get-up | open | 27 | 27 + 4 per special | 14 |
| Lee | 0 / 2 / 3 | crouch, crouching block, crouching hit, throw, get-up | 2 named, none present | 20 | 28 | 10 |
| Gen | 1 / 2 / 3 | crouch, crouching block, crouching hit, throw, get-up | 2 named, none present | 22 | 30 | 12 |
| Birdie | 1 / 3 / 4 | crouch, crouching block, crouching hit, throw, get-up | 2 named, none present | 26 | 34 | 13 |
| Eagle | 2 / 2 / 4 | crouch, crouching block, crouching hit, throw, get-up | 2 named, none present | 26 | 34 | 14 |
| Adon | 0 / 3 / 4 | crouch, crouching block, crouching hit, throw, get-up | 2 named, none present | 24 | 32 | 13 |
| Sagat | 0 / 3 / 5 | jump arc, crouch, crouching block, crouching hit, throw, get-up | low Tiger Shot | 29 | 33 | 13 |

The last column counts the 18 normals (six standing, six crouching, six
jumping) against the distinct normal animations each fighter has (Ryu
12). Each is about 2 new poses.

Where an existing stream could stand in for a named special (Gen's flurry,
Birdie's headbutt, Eagle's barrage and leap, Adon's flying kick), the
"with specials" figure drops by 4 per stand-in.

All twelve together: about 254 new poses without specials, against 442
poses the ROM holds across them (the per-fighter distinct counts above,
summed; Ken's 71 are his own copy). **Estimate** (254), **M** (442).

## Art the walk did not reach

- **Unreferenced streams.** Three streams carry known hit functions, but
  none of the walked tables, records or gotos reaches them: Geki 0x0bd40,
  Joe 0x0e51a and Birdie 0x117fe. Each draws the same poses as a
  referenced stream, so they add no art. The scan finds only streams with
  a hit function; an unreferenced stream without one would be missed.
  **M**
- **Undrawn tiles.** 775 of the 9,440 non-blank `gfx3` tiles in
  0x0000..0x3fff are drawn by no walked fighter pose. In the pictures most
  are effects: hit sparks, projectiles, Geki's smoke and bonus-stage
  debris. Some are fighter art: two standing figures with Geki's mesh
  limbs and raised claws at 0x076a..0x07bf, two gi-wearing figures in
  Ken's block at 0x0869..0x08bf, one Birdie figure at 0x2489..0x24fb, and
  leg-only sections at 0x2bc0..0x2bf7, 0x2f80..0x2ffb and 0x33c6..0x33ff.
  Others I could not identify, for example the dark hand-like pieces at
  0x23a0..0x23f3. Whether a path the probe did not walk (effects, ending,
  bonus stages) draws them is not settled. **M** (counts), **V** (what
  they show)

## Open questions

These decide parts of the inventory, and neither the code nor the docs
settle them.

1. **Does SF1+ need a distinct animation per button strength?** In 1987
   even Ryu shares art: medium and heavy punch, medium and heavy kick, all
   crouching punches, and the jump attacks by direction. The summary gives
   both figures; the "missing" counts depend on the answer.
2. **What are the SF1+ specials for Retsu, Geki, Joe and Mike?** The
   README's SF1+ roster bullet names none. `sf1plus.rs` names Kiai Shot;
   Shuriken and Teleport Strike; Rolling Sobat and Dynamite Straight; Bison
   Rush and Iron Upper. I did not treat the code as the spec.
3. **Which existing streams stand in for named specials?** Candidates
   (by eye): Gen's flurry 0x10b4e for Hyakurenko, Birdie's headbutt
   0x11b42 for Bull Head, Eagle's 0x124d8 and 0x129bc, Adon's flying kick
   for Jaguar Kick. None is documented as such. (For Sagat, `roster.rs`
   already binds 0x14242 as the Tiger Knee.)
4. **Does SF1+ knock fighters down mid-round?** If so, every fighter needs
   a get-up; 1987 has none. `sf1plus.rs` has a `WAKEUP_PUSH` constant,
   which suggests yes.
5. **Is a lose pose wanted?** 1987 has only the knock-out lying pose.
6. **Do the boss jump arcs need a second, diagonal version?** Ryu, Ken and
   Geki have a separate somersault for diagonal jumps; the other bosses
   use one arc for all three.
7. **Can Eagle's second guard `0x2aa4` be his crouching block, and Mike's
   single guard `0x1648` serve both heights?** I could not tell from the
   pictures alone.
8. **What do the undrawn fighter tiles belong to?** See the section
   above.
9. **SF1+ also lists a command heavy for every fighter** (`sf1plus.rs`,
   "command heavy"). It is not in this inventory's categories.

## Discrepancies found in other docs

- `ken-attacks.md` gives Ken's move table as 0x1731c. The ROM and
  `roster.rs` use 0x1662c; 0x1731c is Mike's move 0x0c header. **M**
- `sagat.md` lists stream 0x13ea0 for hit function 0x47eaa. The AI records
  start that jab at 0x13ec0; 0x13ea0 falls inside Sagat's victory stream.
  **M**
- `sagat.md` says 0x47f18 tests the crouching block 0x24. The ROM's
  compare at 0x47f86+2 uses 0x24 and the one at 0x47f18+2 uses 0x22, as
  `fighter.rs` has it. **M**
- `adon.md` calls 0x13850 and 0x1398c leaping kicks. Their frames have
  dy 0; they step forward on the ground. **M**
- Gen's jab 0x103b0 (frame 0x103d6, hit function 0x47234), Sagat's jab
  0x13ec0 (frame 0x13ec6, 0x47eaa) and the last hit frame of Gen's
  0x10a26 (0x10abe) carry a hit function without the active bit (+0x1a
  bit 7), so those frames cannot hit, and the two jabs never hit at all.
  The docs list 0x47234 and 0x47eaa as unexercised, which fits. **M**
