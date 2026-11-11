# Opponent AI tables

This document records the boundary established by
issue #23: the CPU
opponent's setup is translated, for any opponent, difficulty and variant.
The setup is the AI tables the decisions read (0x44880) and the move-table
pointer (0x44860). It is `fighter::Bus::setup_cpu_opponent` and
`ai_table_copies`.

## Where the data comes from

**Pointer arrays.** The starting country's handler (0x3e38a, a jump table to
0x3e39a / 0x3e416 / 0x3e492 / 0x3e50e) points the 15 per-opponent pointer
arrays at A5+0x401c–0x4054 (`0xffc01c–0xffc057`) into ROM at
`0x19908 + 0x28 × country + 0xa0 × n` (`n = 0..14`). Each array holds 10 long
pointers, one per opponent: the move-table pointer (`0x401c`, read by
`0x44860`), the 12 AI-table pointers (`0x4020–0x404c`, copied by `0x44880`),
and the two CPU reaction-table pointers (`0x4050–0x4054`, copied to
`0xffc664 + 0x16` and `+ 0x1a` by `0x44860`).

**Opponent index.** The opponent is stage (0xffc003) × 2 + fight (0xffc004).
Its move-table pointer goes to the CPU structure +0x12 (0x44860), and 0x44880
copies its tables into work RAM 0xffc058–0xffc663. Rows are chosen by:

- **D1:** the difficulty 0xffc007, from DSW2: 0 Normal, 1 Easy, 2 Difficult,
  3 Very Difficult (DIP naming).
- **D2:** the variant, bit 1 of the country byte 0xffc006.

| Pointer | Destination | Words | Row |
| --- | --- | --- | --- |
| 0x4020 | 0x4068 preferred ranges | 3 | — |
| 0x4024 | 0x4058 range classes | 8 | D1·16 + D2·64 |
| 0x4028 | 0x406e distance limits | 3 | — |
| 0x402c | 0x4074 near / middle / far moves | 24 | 0, 0x30, 0x60, 0x90 by D1; +0xc0 for D2, **except at D1 = 0** |
| 0x4030 | 0x40a4 stunned moves | 16 | D1·32 + D2·128 |
| 0x4034, 0x4038 | 0x40c4, 0x40d4 range-shift and react rolls | 8 each | D1·16 + D2·64 |
| 0x403c, 0x4040 | 0x40e4, 0x4164 reactions to airborne / ground attacks | 64 each | D1·128 + D2·512 |
| 0x4044, 0x4048, 0x404c | 0x41e4, 0x4364, 0x44e4 attack records | 192 each | D1·0x180 + (D2 ? 0x600 : 0) |

**The D1 = 0 quirk.** At difficulty 0 the move tables' branch (0x448ee)
leaves before the variant test, so the variant offset is skipped. The
translation keeps this.

**Per-opponent blocks.** Every copy for opponent k reads inside one
0x300e-byte block at 0x1c048 + 0x300e·k, at fixed offsets: classes +0,
ranges +0x80, limits +0x86, moves +0x8e, stunned +0x20e, rolls +0x30e and
+0x38e, reactions +0x40e and +0x80e, attack records +0xc0e, +0x180e and
+0x240e.

## Opponent order

Read from the move-table pointers at 0x19908 + 0x28 × country, listed by
opponent index (stage × 2 + fight):

| Country byte | 0, 1 | 2, 3 | 4, 5 | 6, 7 | 8, 9 |
| --- | --- | --- | --- | --- | --- |
| 0 (Japan) | 0x16e6c, 0x16bfc | 0x1734c, 0x170dc | 0x175bc, 0x1782c | 0x17a9c, 0x17d0c | 0x17f7c, 0x181ec |
| 1 (China) | 0x175bc, 0x1782c | 0x17a9c, 0x17d0c | 0x16e6c, 0x16bfc | 0x1734c, 0x170dc | 0x17f7c, 0x181ec |
| 2 (USA) | 0x1734c, 0x170dc | 0x16e6c, 0x16bfc | 0x17a9c, 0x17d0c | 0x175bc, 0x1782c | 0x17f7c, 0x181ec |
| 3 (England) | 0x17a9c, 0x17d0c | 0x175bc, 0x1782c | 0x1734c, 0x170dc | 0x16e6c, 0x16bfc | 0x17f7c, 0x181ec |

The last stage is the same pair from every start.

**Identities.**

- **Observed:** 0x16e6c is Retsu, the fixed fight's opponent from a Japan
  start.
- **Observed:** 0x175bc / 0x1782c are Lee and Gen (China), and 0x17a9c /
  0x17d0c are Birdie and Eagle (England). In `sf-win-04` to `sf-win-07` the
  VS screens read `RYU VS LEE`, `GEN`, `BIRDIE` and `EAGLE` (frames 19003,
  21868, 25991, 28995), and each fight that follows has that table at
  0xffc676 on the Great Wall, China street, England street and England
  castle stages (frames 19380, 22266, 26372, 29427). The four-country
  select (`sf-select4-01`) shows the China flag for byte 1 and the Union
  Jack for byte 3, the starts that meet those pairs first.
- **Inferred** from the country pairing and the game's known roster:
  - 0x16bfc: Geki.
  - 0x1734c / 0x170dc: the USA pair (Joe, Mike), since the USA start
    (byte 2) meets them first.
  - 0x17f7c / 0x181ec: Adon and Sagat.
- **Not verified here:** the inferred identities against a capture of their
  fight.

## How much difficulty matters

How many of the eight difficulty and variant settings give different
tables depends on the opponent's data (by opponent index):

| Index | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Distinct images | 8 | 8 | 8 | 8 | 5 | 5 | 5 | 6 | 4 | 1 |

The last opponent's rows are the same everywhere in its block, so neither
difficulty nor variant changes his AI tables.

## Verification

The acceptance test is `crates/sf-engine/tests/ai_tables.rs`.

- **Retsu, exactly.** Take the fixture's work RAM (cpu-01, 2389) with the AI
  tables and the CPU move-table pointer wiped. The setup for stage 0,
  fight 0, at the fixture's difficulty and country byte, reproduces the
  reference's 0xffc058–0xffc663 and the pointer 0x16e6c byte for byte.
  They are unchanged at 2433.
- **All 80 settings** (10 opponents × 4 difficulties × 2 variants):
  - each copy reads inside that opponent's own block, at the layout's
    offset, without crossing into the next table;
  - the ten blocks are distinct;
  - the distinct-image counts above hold.

## Not covered

- **The setup's call site.** The round initialisation (0x3e77c, ticket #29)
  runs it in the game. The fixtures already carry the tables, so the native
  machines do not call it yet.
- **Real fights against other opponents, difficulties or variants.** No
  reference capture yet. The scenario harness runs with default DIP
  switches only.
- **The country handlers' other pointer sets** (0x3e416 …): read for the
  order above, not exercised.
