## Problem Statement

The planned forty-fighter roster needs individual native music identities. Older wave plans assign sixteen shared stage themes to twenty-eight bonus challengers, so fighters such as Akuma, Dan, Charlie Nash, and Balrog can lose their own music when sharing a stage. The remaining arrangements also have no consolidated exact-source or listening-acceptance workflow.

Ryu's approved World Warrior arrangement proves native playback with SF1 instruments, but Ryu and the original test theme already occupy 10,828 of the current 16,384 bytes. The remaining 5,556 bytes do not establish capacity for twenty-nine more arrangements. Extending a constant would collide with native sound RAM or truncate addresses.

## Solution

Give each bonus challenger its selected individual character theme, add the chosen World Warrior theme for Ken, and preserve Ryu's approved arrangement and the ten original SF1 opponents' music. Use larger separate logical score storage while retaining native sound-driver playback, SF1 instruments, existing priority/effect/stop behavior, and Faithful 1987 isolation.

Arcade follows the CPU opponent. Multiplayer and Training follow explicit stage associations: one associated participant supplies the theme, two alternate each match, and neither means cycling the stage's associated themes. Existing Jukebox overrides remain available.

Every new arrangement passes a verified source card, an approved short proof, a complete musical loop, separate FM-part review, and native integration. The thirty approved playable scores are tracked in `assets/sf1-port-music`. Source event data and listening artifacts remain local.

### Approved repertoire

All selected sources use original arcade arrangements where available. The inventory is thirty arranged character themes, including the already-approved Ryu arrangement, plus ten retained SF1 opponent themes. The original test theme is additional proof content.

| Source | Recipient to source cue |
|---|---|
| **1991 World Warrior** | Ryu, Ken, Chun-Li, Guile, Dhalsim, Zangief, Vega/claw, Balrog/boxer, and M. Bison/dictator each receive their respective original theme, matching Ryu's World 910522 reference. Ryu is already approved. |
| **Alpha 2 arcade** | Dan, Akuma, Rose, and Charlie Nash receive their own stage themes; Dorai receives Gen's stage theme. |
| **Alpha 3 arcade** | Evil Ryu receives **Black Power**; Gouken receives Ryu's **The Road**; Goutetsu receives Akuma's **Feel the Cool**; Gou Hibiki receives Dan's **Performance**; Zeku receives Guy's **No More Swingin'**. |
| **2nd Impact arcade** | Oro receives his normal stage music as played in that game; verify the exact cue/command before extraction. |
| **1989 Final Fight arcade** | Guy receives **Slum 1**; Cody **Subway Park 1**; Haggar **Slum 2**; Sodom **Subway Park 2**; Rolento **Industrial Area 2**; Hugo/Andore **West Side 1**; Abigail **Bay Area 3**; Poison **Bay Area 1**. |
| **Final Fight Revenge arcade** | Damnd receives **DAMND**; Belger receives **BELGER**. |
| **Retained SF1** | Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, and Sagat retain their native themes. |

The borrowed family/teacher assignments are deliberate project choices. They do not claim to be the recipients' canonical themes. Normal fight arrangements are the default; different editions, console arrangements, urgent variants, and different compositions are not silently substituted.

## User Stories

1. As a player, I want thirty arranged character themes and ten retained SF1 opponent themes, so that the planned forty-fighter roster has complete music coverage.
2. As a player, I want each character's approved source cue and arcade arrangement, so that another edition or composition is never substituted silently.
3. As an original-roster player, I want the ten SF1 opponents' existing music retained, so that the original stage identities remain intact.
4. As a maintainer, I want stable theme identities independent of filenames, load order, localized names, and runtime command allocation, so that dispatch remains unambiguous.
5. As a maintainer, I want Ryu's established command, the original test theme, and supported scores preserved while the obsolete Evil Ryu reservation is replaced, so that the new library does not break existing material.
6. As a player, I want music work to respect released-wave gates, so that it never exposes unfinished fighters, stages, or modes.
7. As an arranger, I want a source card naming the recipient, source character/cue, game, platform, edition, region, revision, and sound version, so that the intended arrangement is reproducible.
8. As an arranger, I want required local resources, archive/member hashes, and scope of use verified before dependent extraction, so that a filename or past audit is not mistaken for source evidence.
9. As an arranger, I want a reproducible cue command, data association, or in-game trigger, so that a reviewer can confirm the exact music selected.
10. As a reviewer, I want the full introduction, tempo/time base, true loop boundaries, sustained state, and two reference passes recorded, so that fades and arbitrary capture edges do not become loop data.
11. As an arranger, I want each source sound system investigated independently, so that an SF2-specific extractor is not assumed to support another game.
12. As a planner, I want verified resources distinguished from unresolved sourcing dependencies, so that a bounded unsuccessful search is not treated as proof of absence.
13. As a maintainer, I want the thirty approved playable scores in the repository so that a source checkout can play them with a supplied ROM set. Extracted events, captures, and listening artifacts stay local.
14. As a listener, I want a short representative proof exposing lead, bass, harmony, register, timing, and accompaniment, so that I can judge the direction before the full arrangement is developed.
15. As a reviewer, I want voice merging, omitted percussion, register displacement, and other material compromises explained, so that source selection is not mistaken for approval of musical simplification.
16. As a reviewer, I want to approve the short proof before complete expansion, so that the larger arrangement follows an accepted direction.
17. As a listener, I want the complete source introduction and phrase order with a true loop and two full passes, so that the arrangement works beyond an attractive excerpt.
18. As a reviewer, I want the complete mix and useful named individual FM parts at consistent reference levels, so that balance can be judged part by part.
19. As a reviewer, I want to revise instruments, attenuation, modulation, feedback, and balance before acceptance, so that the final arrangement reflects listening judgment.
20. As a listener, I want Ryu's approved balance retained without imposing it on every other theme, so that each arrangement receives its own musical judgment.
21. As a reviewer, I want percussion and necessary voice reduction decided during proof review, so that hardware compromises are explicit and intentional.
22. As a player, I want approved music heard through native playback, so that an offline audition alone does not stand in for real integration.
23. As a listener, I want an individual score over 16 KiB and a library over 64 KiB to play correctly across old address boundaries, so that faithful arrangements are not shortened to fit the old window.
24. As a player, I want native sound ROM/RAM meanings preserved without a new storage setting, so that capacity work does not alter the original machine's behavior.
25. As a maintainer, I want independent actionable failures for invalid scores, positions, jumps, and zero-time loops, so that one bad entry cannot hang playback or invalidate good themes.
26. As an arranger, I want supported readable controls and existing scores retained, so that Ryu's installed score and the original test material remain usable.
27. As a listener, I want higher-, lower-, and equal-priority behavior preserved across native and extended tunes, so that the new storage does not change musical interruption rules.
28. As a listener, I want saved themes to pause or advance silently according to the existing driver behavior, so that they resume at the correct musical state.
29. As a listener, I want effects to return their channels to the correct theme state, so that sound effects do not damage the arrangement after they finish.
30. As a listener, I want correct stops, discards, restoration, gates, held pitch, voice controls, and loops for current and saved themes, so that larger storage preserves the approved native controls.
31. As a player, I want library replacement or disabling to retain live streams and pending requests safely, so that neither wrong-library playback nor lost cues occurs.
32. As a listener, I want elapsed-chip-clock timing preserved under eight-voice load and repeated loops, so that compilation or file access does not add drift to playback.
33. As a Faithful player, I want unchanged original behavior and no music-pack access in Faithful fights, the attract, or the faithful cabinet, so that overhaul music stays isolated.
34. As an Arcade player, I want the CPU opponent's character theme regardless of stage, so that the opponent's identity determines the music; accepted CPU-led encounters use their designated opponent or encounter owner.
35. As a Local or Training player, I want the sole participating stage member's theme when exactly one distinct participant belongs to the stage, so that the stage's present fighter supplies its music.
36. As a Local or Training player, I want two participating stage members to alternate by identity each match, beginning with P2 for a new sequence, so that both receive their own theme.
37. As a Local or Training player, I want a stage's associated themes cycled when neither participant belongs, so that the stage retains its musical identity through its own cast.
38. As a mirror-match player, I want one distinct fighter counted once, so that a mirror does not invent two different themes.
39. As a player, I want stage cycles to begin with the primary owner and then roster order, with separate phases per stage/candidate sequence, so that the sequence is predictable.
40. As a player, I want rounds, pause, Training reset/reposition, and resimulation to retain the current choice, while new matches advance it, so that musical rotation follows matches rather than incidental events.
41. As a player, I want sequence phases retained through Rematch and Reselect and cleared at the main menu, so that a session has a consistent musical sequence.
42. As a player, I want started matches counted despite Jukebox overrides or unavailable scores, while cancelled setup does not count, so that local files do not change future choices.
43. As a player, I want an unavailable arrangement to use the established SF1 stage fallback with a useful diagnostic, so that a missing file never blocks a fight.
44. As a Jukebox user, I want existing override precedence, competing-track behavior, and NATIVE handling preserved, so that explicit settings continue to work with the newly selected native theme.
45. As a replay or online player, I want the selected identity, timestamp, and sequence phase retained through resets, snapshots, replay, and rollback, so that resimulation neither rotates twice nor replays already-heard audio.
46. As a player, I want gameplay outcomes and checksums independent of local score availability and library bytes, so that presentation assets do not affect the fight.
47. As a reviewer, I want public synthetic checks, compatibility evidence, real front-end playback, measured sizes, and recorded listening approvals, so that missing-resource skips and historical results are reported honestly.

## Implementation Decisions

- **Larger native theme library.** Keep the original sound CPU memory map intact and provide checked logical storage for extended scores. Support larger individual scores as well as larger aggregate libraries. The exact cursor/bank representation and compiled layout are builder decisions.
- **One playback owner.** The native audio player owns addressing, library lifetime, current/saved channels, loop state, held pitch, gates, voice controls, timing, priority, effects, stop/discard, and handback. Source identity and musical state move together through every lifecycle operation.
- **Deterministic engine selection.** The engine owns the character-theme registry, stage associations, selected match identity, and sequence phase. The host loads and compiles resources before playback requests and performs no file reads or compilation in the audio callback.
- **Timed identity handoff.** Carry the selected semantic identity with the timestamped request through engine output, queues, resets, and rollback bookkeeping. Enablement flags or reused command bytes cannot ambiguously identify different themes.
- **Command compatibility.** Preserve Ryu's `0x41` and the test theme's `0x40`; allocate the remaining commands centrally and explicitly reconcile the old Evil Ryu collision. The existing extended command range is sufficient for this inventory.
- **Native compatibility.** Existing supported score controls and legacy inputs remain valid. ROM fetches, native instruction costs, priority semantics, paused versus silently advancing saved themes, and effect ownership retain their established behavior. Faithful paths open no extended library or pack.
- **Stage membership.** Preserve the accepted shared-stage groups. Primary ownership identifies the listed stage; membership determines native music. Ryu's and Ken's cursor defaults do not associate them with Retsu's and Joe's stages.
- **Rotation defaults.** Two-member alternation begins with P2, keyed by stage and unordered fighter identities. Stage cycles begin with the primary owner followed by roster order. Keep phases per stage/candidate sequence within the session; changing candidates creates a new sequence and returning to an existing sequence resumes it. Advance only the sequence rule used by a started new match; clear phases at the main menu.
- **Other contexts.** Supported online two-player matches use the same stage-based rule. CPU-led exhibitions and World Tour encounters follow the designated CPU opponent, or the encounter owner/boss for multiple CPU opponents. Do not implement an unavailable mode solely for this work.
- **Replay presentation state.** Restore logical selection and phase with the session/replay; exclude music presentation state and local library data from gameplay checksums. Deduplicate audible events already delivered during rollback.
- **Jukebox and fallback.** Preserve explicit override precedence, including NATIVE blocking lower-priority file choices, and existing resolution of competing external fighter tracks. Select identity before checking local availability. Missing or invalid scores use the existing native stage fallback; a new stage lacking an original tune retains `0x29`. Availability does not skip or reorder cycle members.
- **Exact sources.** Prefer source music data to transcription, verify each selected arcade set/cue independently, record hashes and musical loop evidence, and expose the exact pin at the first proof. Normal fight arrangements are the approved default. Missing resources remain sourcing dependencies, never permission for silent substitutions.
- **Listening acceptance.** A short proof is approved before full expansion. The complete arrangement preserves source structure and two real loop passes; the mix and individual FM parts are reviewed and adjusted before acceptance. Offline rendering is an audition aid, not native-integration evidence.
- **Ryu baseline.** Preserve its approved stock SF1 lead, bass, and lead echo, with gentler/quieter accompaniment. This remains a theme-specific result, not a mandatory preset for other fighters.
- **Percussion.** Decide per proof. Ryu remains FM-only. FM percussion shares the voice budget; new synchronized ADPCM requires an agreed native timing/stop/resume/ownership design before implementation. Do not add a host-played percussion bed automatically.
- **Pack distribution.** Track the thirty approved playable scores in `assets/sf1-port-music`. Load the repository pack from the executable or working directory's `assets` folder and accept valid ROM-adjacent overrides by theme identity. Keep the title arrangement in the ROM-adjacent folder. Extracted events, source captures, proof and review recordings, and source-specific evidence remain local beside the ROM and reference resources. Windows ZIP packaging continues to include the art and voice packs.

## Testing Decisions

Use the external seams already accepted in the design. A good automated test supplies player/session inputs, scores, and timed music requests, then observes selected identities, sound events, chip writes/output, diagnostics, and replay behavior. It does not mirror private representation choices or treat an automated assertion as musical approval.

1. **Real game session through native audio.** Drive the existing front-end/session seam into a fight and observe the selected identity and native output. Cover all stage-association cases, Arcade opponents, mirrors, side swaps, Rematch/Reselect, cancelled setup, Training reset, fallback, Jukebox overrides, exits, and subsequent Faithful play. Use fixtures for unreleased stage groups without releasing their waves.
2. **Native timed playback from power-on.** Use original synthetic scores for a single score over 16 KiB, positions beyond the old 16-bit boundary, and a library over 64 KiB. Verify loops/jumps and invalid input behavior; higher/lower/equal priorities; paused and silently advancing saved music; native/extended transitions; effect handback; both stop/discard slots; replacement with live saved streams; and timing under eight-voice load. Prior art is the existing extended-tune, raw-control, stop, and sound-test coverage.
3. **Replay/rollback through session audio delivery.** Restore snapshots and replay/resimulate inputs while observing identities, phase, timestamps, and delivered sound. Verify no duplicate selection or audible restart, and identical gameplay outcomes/checksums despite missing local music.
4. **Local arrangement acceptance.** For each of the twenty-nine new arrangements, retain the source card, reference evidence, approved short proof, approved complete loop/mix and FM parts, measured compiled size, and native installation/playback results. Reuse the five family-first proofs in their recipient's arrangement ticket rather than recreating them.

Public checks use original material. ROM-dependent checks explicitly skip unavailable private resources. Ryu's historical 59 targeted checks and 112-second native probe are prior evidence, not tests rerun by this planning session.

## Out of Scope

- Fighter movesets, stage art, roster-wave release gates, or implementing unfinished modes.
- Changes to Faithful 1987, the attract, the faithful cabinet, or the approved Ryu arrangement.
- Replacing native themes with rendered Jukebox audio or redesigning Jukebox controls.
- Automatically composing extra original themes to satisfy stale shared-stage ticket wording.
- Assuming one universal importer, assuming an archive name proves the source, or substituting console/other-edition music without review.
- Automatically adding percussion or a new synchronized ADPCM backend.
- Claiming subjective listening acceptance from code tests or marking an arrangement complete before its required human reviews.

## Further Notes

The user approved the full character-theme design and its author-proposed defaults on 2026-11-10. ADR-0023 records the architecture; ADR-0009 is amended to retain native playback while replacing the fixed storage-window assumptions. The project keeps domain vocabulary in CONTEXT rather than a separate glossary.

The earlier wave-theme issues [#31](https://github.com/johnstreetfighter/sf1-port/issues/31), [#49](https://github.com/johnstreetfighter/sf1-port/issues/49), and [#62](https://github.com/johnstreetfighter/sf1-port/issues/62) need reconciliation with individual themes, the approved arrangements, stage membership, and the command collision. This spec does not silently add the original shared-stage compositions described there.

Resource findings are bounded: World Warrior archives and Ryu evidence were found; Alpha archives are documented historically but their current location was not found; 2nd Impact archives/SIMMs/CHDs exist but require an exact audit and cue investigation; original Final Fight and Revenge/ST-V resources were not located. These are investigation prerequisites, not proof that copies do not exist elsewhere. No source extraction or music-asset work occurred during planning.

Core storage/selection work can use original synthetic material and approved local Ryu content while source-family investigations proceed independently. World Warrior is the first new arrangement family because it is closest to established evidence. Other source-family order follows resource readiness and listening availability. Every arrangement remains subject to the short-proof, full-mix, and individual-part review gates.
