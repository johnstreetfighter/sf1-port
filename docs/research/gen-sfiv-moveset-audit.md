# Gen moveset audit for original Street Fighter IV and Arcade Edition 2012

Source evidence for [#148](https://github.com/johnstreetfighter/sf1-port/issues/148). The original SFIV card in the port supports Gen. Capcom introduced him as a console-exclusive playable character and the original Windows retail depot includes his common package. The authenticated 2008 arcade file inventory has no Gen common package. The original timing column uses Capcom's 2009 PC benchmark character data. Retail byte identity and paired live play remain open. [Capcom's February 2009 preview](https://news.capcomusa.com/lets/browse/street-fighter-iv-strategy-guide-preview) · [Original PC release](https://store.steampowered.com/app/21660/Street_Fighter_IV/) · [Retail depot file manifest](https://steamdb.info/depot/21661/)

## Source builds

The original benchmark installer `StreetFighterIV_Benchmark.exe` is 416396564 bytes with SHA-256 `4844d66b39b934a15c959d9227fd4ff1d58dc8d4365f8f060e784dc239f8b19b`. Extract the benchmark's `Battle/Chara/GEN/GEN.cmn.emz`, raw-inflate after the 16-byte EMZ header, then select the two named EMB members. The original 2008 arcade comparison used Taito Type X2 v1.0 STD, archive SHA-1 `1120092ad2dcde6a381a2305dbbb843339585c44`, launcher MD5 `e47d7c83e526dbb1986173f197801b99`. [Benchmark distribution listing](https://game.watch.impress.co.jp/docs/news/294104.html) · [Arcade preservation metadata](https://archive.org/metadata/Yori-Arcade-Arcade_Games_-_TAiTO_Type_X2-RomVault) · [Original board fingerprint](https://github.com/teknogods/OpenParrot/blob/fc8ba632e1f67c5c340ff88f30efca743e8d38f5/OpenParrot/src/Utility/GameDetect.cpp#L322)

AE2012 uses Japanese Taito Type X2 NESYS 1.3, archive SHA-1 `dd2d5cf64e9d35dd1b3da772c9d72b906b860026`, `game.exe` MD5 `15432af2c3ec96b7dec196477b0e3591`, normalized header CRC-32 `545b0d0c`. Use `patch/battle/regulation/latest/GEN/`. The same archive's `resource/battle/chara/GEN/` files provide an earlier balance comparison. There are no Gen files in `v101/`. [AE2012 board fingerprint](https://github.com/teknogods/OpenParrot/blob/fc8ba632e1f67c5c340ff88f30efca743e8d38f5/OpenParrot/src/Utility/GameDetect.cpp#L449) · [Capcom AE2012 changes, page 6](https://static.capcom.com/streetfighter/downloads/SSFIV%20Arcade%20Edition%202012%20Final%20Change%20List%20-%20US.pdf#page=6)

| Member | Bytes | SHA-256 |
|---|---:|---|
| Benchmark `GEN.cmn.emz` | 10629406 | `c75c8815e602e68eb38c954100e634d869e9121fb78fd4f8694551135d5886b2` |
| Benchmark `GEN.bac` | 296784 | `3b6eecb90e6ae0501622f1fac092d794d6a8cde1e03b26e0ebf194442e68e48c` |
| Benchmark `GEN.bcm` | 16925 | `4ebf580caac2ca849d0e4bfbd5c6646de78deeb56c042f7b9214fcf22e431728` |
| AE2012 latest `GEN.bac` | 339370 | `2cbc851a495ebc27ccaccfdc1ff6708be79295b13984d0717860d982ac1b6bb0` |
| AE2012 latest `GEN.bcm` | 20478 | `ed0bb4a80c333630899aa1ad6a4b56ab56af59b2358f39795222171c05c7c0da` |
| Arcade resource `GEN.bac` | 314421 | `74f5b1123df75ede2a73d4b4054d82a231691c7c5095d93fc7258aa359028942` |
| Arcade resource `GEN.bcm` | 18603 | `b9042b27b1b7dfa4adcc0e2b43fa8ee035ee8dabdfafed70867a5bf0d9995e16` |

The original benchmark has 349 populated main action slots, 141 reactions, 140 commands, nine motion definitions and 78 cancel lists. AE2012 latest has 387 populated main slots, 160 reactions, 156 commands, 12 motions and 84 cancel lists. Retain numeric BAC IDs when inspecting Gen. Mantis and Crane both use names such as `FORWARD`, `5LP` and `9HK`. The loaded stance selects the parallel normal and movement actions at an offset of 1000. A decoder keyed only by action name loses Mantis rows. [Pinned newer-format BAC and BCM decoder](https://github.com/dantarion/sf4tools/tree/e5c17132f3b6e63f66150c3a347a9e8f6e0d73a4/SFUltraDiff/src)

## Input inventory

Directions are facing-relative numpad notation. Command row indices are zero-based.

| Original / AE2012 rows | Inputs and variants |
|---|---|
| 0–35 | Per stance, six close/far standing buttons at 1.2 native units, six crouching buttons, six neutral-jump buttons and six directional-jump buttons. Rapid LP/LK have separate close/far and crouching routes. |
| 36 / 36–39 | Crane airborne HK into a second fresh HK via `HKK`. Latest AE2012 adds Mantis close/far HP → MK and 2LK → 2MK. A third `TC_5HP` endpoint is present, with no source action referencing its cancel list. |
| 37–38 / 40–41 | Both stances use `66` and `44`. |
| 39–46 / 42–49 | Mantis Hyakurenko. LP/MP/HP mash, L/M/H and EX, with separate initial and continuation commands. Five lenient P samples are encoded with per-sample buffer 15 in AE2012. |
| 47–50 / 50–56 | Crane Jyasen. `[4]6P` with L/M/H and EX. Original has one EX route. AE2012 separates LP+MP, LP+HP, MP+HP and PPP, with PPP sharing MP+HP's BAC. |
| 51–61 / 57–74 | Crane Oga. `[2]8K`, L/M/H and EX, with wall and high-arc choices. The first set is back feint, forward far kick or up high arc. The high arc has forward and backward kicks, down drop and up feint. AE2012 gives EX separate versions of these seven choices. Both charge definitions specify 60 charge frames in both extraction sets. |
| 62–120 / 75–133 | Mantis Gekiro. `623K` with LK/MK/HK or KK. Ordinary variants take corresponding fresh kicks, with multiple timing-dependent follow-up rows in each stage. Total stage indices reach six, seven and eight for L/M/H, and eight for EX. |
| 121–126 / 134–139 | Mantis Zan'ei `236236P` and Crane Jyakoha `236236K`, three strength rows each. Cost 1000 raw Super units. |
| 127–128 / 140–144 | Ultra I is Mantis Zetsuei `236236PPP` and Crane Ryukoha `236236KKK`. AE2012 adds Ultra II Mantis Shitenketsu `214214PPP` and Crane Teiga `236236KKK` in the air. Teiga requires source high restriction 0.5. Zan'ei's hit script offers a distinct Shitenketsu cancel route. The Ultra threshold and cost are 200/1000 raw Revenge units. |
| 129–135 / 145–151 | Forward/back throw, Focus, EX Focus, two early releases, and appeal. Focus uses MP+MK, EX Focus costs 500 raw Super, appeal uses HP+HK. |
| 136–139 / 152–155 | From Crane, PPP selects Mantis. From Mantis, KKK selects Crane. Each direction has distinct standing and crouching actions. The source rows encode press and release; the switch scripts have raw IASA 5 and total 30. |

## Initial timing and balance checks

Intervals are half-open local BAC action ticks. Raw script coordinates are distinct from elapsed frames where the source changes animation rate, freezes or branches.

| Property | Original PC benchmark | AE2012 latest |
|---|---|---|
| Forward / backward walk | Mantis +0.045 / −0.030 native units per advancing tick. Crane +0.055 / −0.045. Acceleration 0. | Same source velocity pairs. |
| Crane double jumping HK | Neutral or directional first HK with the `HKK` cancel routes at `[6,19)`; target action `[4,6)`, 50 damage and 50 stun, stop 12/12. | Preserves the selected input and active windows. Check hit, guard, whiff and diagonal direction in a live trace. |
| Crane sweep | Raw hitbox `[9,11)`; 120 damage/150 stun, stop 12/12, push 0.7. | Raw `[9,15)`. The arcade resource predecessor ends at 13. Capcom describes the AE update as three to five active gameplay frames. |
| Mantis close HP | First strike `[9,11)` and second `[11,15)`, 80/60 damage and 100 stun each. Special cancel execution sits on the first strike. | The same main action windows and first-hit special cancel persist. Damage becomes 70/40. Both close and far HP gain the `TC_5MK` cancel target. |
| Crane crouching MK | `[8,11)`, 80 damage/100 stun, stop 10/10, push 0.5. Super cancel execution `[8,11)`. | At `[1,2)` Crane action 1272 unconditionally enters `2MK_aer` (1286). Its strike is `[7,10)`, with 80 damage/100 stun and stop 10/10. Raw IASA is 30 and the Super cancel executes `[7,10)`. |
| Ultra I opening | Zetsuei has opening `[123,134)` into a capture script. Ryukoha uses an air-only opening `[100,108)` into a capture script, with separate no-hit landing. | Raw opening windows persist. |
| Ultra II opening | Character command table has neither Ultra II route. | Shitenketsu raw hit windows start at 97, 105, 113 and 121, leading to one capture action. The superflash ends at raw 90 instead of the resource script's 88, consistent with Capcom's post-freeze startup change from 9 to 7. Zan'ei's capture has a Shitenketsu cancel prepare window `[37,54)` and execute window `[49,54)`. Teiga can branch to hit or landing over `[70,83)`. Its scripted velocity at tick 68 changes from +0.18/−0.04 in the resource script to +0.28/−0.06. |

The latest Gekiro medium and heavy final-kick command costs change from 0 to −10 raw Super, matching Capcom's completion bonus. Capcom also specifies removal of quick standing on the final heavy kick. The EX Jyasen LP+MP, LP+HP and MP+HP rolling actions carry two, three and four strike groups and end at raw 16, 32 and 47. Each enters its own finishing action with identical sampled strike data at `[5,9)`, 80 damage/50 stun and 14/14 stop. Capcom describes Jyasen hit and guard stun changes for EX Focus dash. EX Oga routes to separate 150-damage near and far kick actions, versus the resource's shared 100-damage actions. The EX far kick sets +0.22 native velocity at raw 10, versus the resource shared far kick's +0.17 at raw 5. Its action speed changes at raw 15 and 23, consistent with the patch notes' 12→7 gameplay startup after the wall bounce. [Capcom AE2012 changes, page 6](https://static.capcom.com/streetfighter/downloads/SSFIV%20Arcade%20Edition%202012%20Final%20Change%20List%20-%20US.pdf#page=6)

Mantis HP → MK uses buffer/execute conditions 6/1 at `[9,11)` on close/far HP (258/264), entering action 309 with a mid attack at `[7,10)`, 40 damage/100 stun and stop 10/10. Mantis 2LK → 2MK uses conditions 6/1 at `[5,9)` on action 271, entering action 308 with a low attack at `[12,15)`, 70 damage/100 stun and stop 10/10. Both finishers have a `SUPER MOVE` execute row over their own strike. Neither set of cancel rows exists in the benchmark or the arcade resource data. Port and cabinet contact, guard, input precedence and whiff behavior remain open.

## Port comparison

The port dispatches Mantis and Crane specials, Super, Ultra and opposite-stance inputs through distinct command lists. It sets the Gen stance immediately when its shared switch move starts. The SFIV bank uses family-wide aliases for these Gen command slots. Check each source stance's walk, jabs and other normal geometry, standing and crouching transition timings, the Crane double HK, Gekiro interval and kick-strength gates, and each Jyasen/Oga branch through ordinary inputs before claiming moveset parity. Existing SFIV native tests cover a Gen Super and Ultra loadout selection, including Crane Teiga. [Port Gen commands](../../crates/sf-engine/src/era_fight/sf4.rs#L323) · [Stance change](../../crates/sf-engine/src/era_fight/sf4.rs#L824) · [Move bank](../../crates/sf-engine/src/data_bank/sf4.rs#L44) · [Native tests](../../crates/sf-engine/tests/sf4_native.rs#L657)
