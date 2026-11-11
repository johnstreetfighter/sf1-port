# Character themes: source inventory, match selection, and larger native score storage

- **Date:** 2026-11-10
- **Status:** Approved by the user on 2026-11-10, including the author-proposed defaults
- **Builder check:** A fresh reviewer found no important unanswered questions needed to plan the work. This is a planning review, not implementation or musical validation.
- **Scope:** Approved planning design. This planning pass makes no arrangement extraction, production implementation, or changes to the approved Ryu arrangement.
- **Context:** [CONTEXT.md](../../CONTEXT.md), [stages and music](2026-11-03-stages-and-music-design.md), [roster](2026-11-03-bonus-roster-by-lore-design.md), [ADR-0009](../adr/0009-stages-and-themes-in-extended-banks.md), [ADR-0017](../adr/0017-forty-fighter-roster-and-satsui-awakening-on-the-1987-engine.md), [ADR-0020](../adr/0020-wave-2-bloodlines-airborne-specials-and-shared-stage-owners.md), [ADR-0021](../adr/0021-wave-3-final-round-360-grapples-wall-dives-and-zeku-form-shift.md), [extended tunes](../reference/extended-tunes.md).

## 1. Intent

Give every bonus challenger an individual native theme, including fighters who share a stage. Preserve the source arrangement as closely as the SF1 sound hardware and instrument palette allow. Make the family and teacher connections audible through deliberately assigned, otherwise unassigned themes where a fighter needs a source.

The inventory covers the planned forty-fighter roster: thirty arranged character themes, comprising the approved Ryu arrangement and twenty-nine new arrangements, plus the ten retained SF1 opponent themes. The original test theme is additional proof content, not a roster theme. Source selection does not release an unfinished roster wave.

The user approved the source choices in section 3, the match rules in section 4, and the larger separate score-storage approach in section 5 during the brainstorming discussion, then approved the complete design and its defaults. The defaults retain their author-proposed provenance and are now approved decisions.

## 2. Goals and boundaries

### Goals

- Preserve melody, bass, harmony, phrase order, timing, register, held-pitch motion, note gates, and musical loops wherever the intended hardware permits.
- Use SF1-derived instruments, played through the native translated sound driver and YM2151, with the existing priority, saved-theme, effect-ownership, and stop behavior.
- Prefer source music data over transcription when a verified, authorized local source permits it. Investigate each sound system independently.
- Support a whole library, and an individual arrangement, larger than the present 16 KiB extended tune window.
- Keep source identity, match selection, and rotation deterministic. Local file availability must not change which theme is logically selected.
- Review each arrangement as a short proof, a complete arrangement, and separate FM parts before accepting it.
- Keep Capcom arrangements, extracted event data, ROM-derived captures, and listening artifacts in the local music pack/reference area.

### Non-goals

- Changing Faithful 1987, its ROM music, the attract, or the faithful cabinet.
- Reworking Ryu's approved musical balance or replacing native playback with his audition WAV.
- Creating new stages, releasing roster waves, changing fighter moves, or redesigning the Jukebox.
- Adding every later soundtrack version, alternate costume theme, or urgent-music variant.
- Promising a universal CPS1, QSound, CPS3, or ST-V importer before source-specific investigation.
- Automatically adding percussion or starting a new ADPCM playback backend.
- Implementing roster modes that do not yet exist just to deliver this music work.

### Failure conditions to avoid

- Simplifying a source silently to fit storage, moving a part an octave without review, or losing accompaniment to make the lead sound cleaner.
- Treating a soundtrack fade as a musical loop boundary.
- Pulling storage out from under a current or saved tune, dropping a new theme request, or restarting audible music during rollback.
- Committing Capcom note/event data or assuming an archive filename proves its revision, availability, or suitability as an extraction source.

## 3. Approved source inventory

The standing preference is the **original arcade version of the chosen music wherever available**. The choices below specify the repertoire and arcade platform; section 6 specifies how the exact source is pinned before an arrangement is made.

### Retained SF1 music

Retsu, Geki, Joe, Mike, Lee, Gen, Birdie, Eagle, Adon, and Sagat keep their native SF1 themes. The existing stage-record rules define these associations. Dorai borrowing Gen's Alpha 2 theme does not replace Gen's SF1 music.

### Street Fighter II: The World Warrior, 1991

| Recipient | Source cue |
|---|---|
| Ryu | Ryu's theme; already approved and integrated |
| Ken | Ken's theme |
| Chun-Li | Chun-Li's theme |
| Guile | Guile's theme |
| Dhalsim | Dhalsim's theme |
| Zangief | Zangief's theme |
| Vega | Claw fighter's theme |
| Balrog | Boxer’s theme |
| M. Bison | Dictator’s theme |

Use the original arrangements matching Ryu's source, not Champion Edition, Hyper Fighting, Super, Super Turbo, or a console arrangement. Ryu's established reference is **World 910522**, sound member `sf2_9.12a`, SHA-1 `7b6bf59dfd578bfbbdb64c27988796783442d659`. Carry this as the reference anchor when verifying the other eight source cues. Western character identities must be explicit because the Japanese boss names differ.

### Street Fighter Alpha 2, arcade

| Recipient | Source cue |
|---|---|
| Dan | Dan's stage theme |
| Akuma | Akuma's stage theme |
| Rose | Rose's stage theme |
| Charlie Nash | Charlie/Nash's stage theme |
| Dorai | Gen's stage theme, deliberately borrowed through their connection |

These are the original Alpha 2 arcade arrangements. Alpha 1, Zero 2 Alpha, Alpha 2 Gold, and console arranged soundtracks are not interchangeable defaults.

### Street Fighter Alpha 3, arcade

| Recipient | Source cue | Connection |
|---|---|---|
| Evil Ryu | **Black Power** | His selected theme |
| Gouken | Ryu's **The Road** | Ryu's master |
| Goutetsu | Akuma's **Feel the Cool** | Teacher of Akuma and Gouken |
| Gou Hibiki | Dan's **Performance** | Dan's father |
| Zeku | Guy's **No More Swingin'** | Guy's teacher |

The four borrowed assignments are intentional project choices. They do not claim to be these recipients' canonical themes. Gouken's SFIV and Zeku's SFV themes were alternatives, not the selected sources. Do not substitute Shin Akuma's **Groan** for **Feel the Cool**. Pin an arcade release containing the selected music; Upper, MAX, and home arrangements are not silently substituted.

### Street Fighter III: 2nd Impact, arcade

| Recipient | Source cue |
|---|---|
| Oro | Oro's normal stage music as played in 2nd Impact |

The game choice is settled. The exact source cue title, command, and event-data association require verification in the chosen set. Do not invent a title or silently use New Generation or 3rd Strike music.

### Final Fight, original 1989 arcade

| Recipient | Selected cue | Published collection identifier |
|---|---|---|
| Guy | **Slum 1** | `ROUND1 SLUM1/ROUND4/ROUND6` |
| Cody | **Subway Park 1** | `ROUND2 SUBWAY PARK1/ROUND6` |
| Haggar | **Slum 2** | `ROUND1 SLUM2` |
| Sodom | **Subway Park 2** | `ROUND2 SUBWAY PARK2` |
| Rolento | **Industrial Area 2** | `ROUND4 INDUSTRIAL AREA2/ROUND6` |
| Hugo / Andore | **West Side 1** | `ROUND3 WEST SIDE1` |
| Abigail | **Bay Area 3** | `ROUND5 BAY AREA3/ROUND6` |
| Poison | **Bay Area 1** | `ROUND5 BAY AREA1` |

These are stage-cue assignments to individual fighters. Slashes in the collection names denote reuse of one piece, not additional compositions. Do not substitute Final Fight CD, SNES, X68000, or arranged versions. Industrial Area 1 and Bay Area 2 remain unassigned; they are not extra required arrangements.

### Final Fight Revenge, arcade

| Recipient | Selected cue |
|---|---|
| Damnd | **DAMND** |
| Belger | **BELGER** |

The user approved Revenge as a fallback within the Final Fight family and explicitly chose these two arcade themes. A Saturn recording may help compare compositions but is not proof of the selected arcade arrangement.

### Identification references

Repertoire titles were checked against the licensed [Alpha 1](https://www.lacedrecords.co/products/street-fighter-alpha-warriors-dreams-deluxe-double-vinyl), [Alpha 2](https://www.lacedrecords.co/products/street-fighter-alpha-2-deluxe-double-vinyl), [Alpha 3](https://open.spotify.com/album/75HAaD2hA74hNAWwSGmQrW), and [Final Fight collection](https://www.cdjapan.co.jp/product/CDGM-10026) listings. [Capcom's collection announcement](https://www.4gamer.net/games/158/G015809/20141020010/) distinguishes the arcade games and ports. Album metadata identifies repertoire; exact game data and loop behavior still require local verification.

## 4. Theme selection in a match

### User-approved rules

| Mode / stage relationship | Native theme choice |
|---|---|
| Arcade | The CPU opponent's theme, regardless of the stage |
| Multiplayer or Training: one participating fighter belongs to the stage | That fighter's theme |
| Multiplayer or Training: both participating fighters belong to the stage | Alternate their themes each match |
| Multiplayer or Training: neither participating fighter belongs to the stage | Cycle through the stage's associated fighters' themes each match |

For example, Ryu versus Ken on the Gouken dojo cycles the themes assigned to Gouken, Akuma, and Goutetsu. Gouken versus Akuma there alternates those participants' themes. Gouken versus Ryu there uses Gouken's theme. A mirror has one distinct participating fighter and one theme, not two different rotation entries.

Stage association means the explicit character-membership list, not the cursor's home-stage alias. Ryu's cursor default on Retsu's stage and Ken's on Joe's do not make them owners or members of those stages. A 1987 stage has its original opponent as its member.

### Stage membership from the accepted wave plans

| Stage / primary owner | Associated fighters |
|---|---|
| Evil Ryu | Evil Ryu |
| M. Bison | M. Bison |
| Gouken dojo | Gouken, Goutetsu, Akuma |
| Gou Hibiki shrine | Gou Hibiki, Dan |
| Dorai courtyard | Dorai, Chun-Li |
| Guile | Guile |
| Dhalsim | Dhalsim |
| Oro | Oro |
| Damnd / Metro City Slums | Damnd, Cody, Guy |
| Haggar / underground ring | Haggar, Hugo, Poison |
| Sodom / subway | Sodom, Rolento |
| Belger / penthouse | Belger, Abigail |
| Zeku | Zeku |
| Rose | Rose, Charlie Nash |
| Zangief | Zangief |
| Vega / golden cage | Vega, Balrog |

Membership does not require releasing an unavailable fighter or stage. The existing released-wave gates continue to control player-visible content.

### Approved defaults for rotation and other contexts

- Resolve the native choice once when a new match actually starts. New rounds, pause/resume, Training reset/reposition, and rollback resimulation keep that match's choice. A new Training match/setup or a rematch advances the applicable sequence.
- Begin a two-fighter sequence with P2's theme. Key the sequence by stage and the unordered pair of fighter identities; swapping sides does not erase an existing sequence. Alternate identities, not whichever side currently holds them.
- Begin a stage cycle with the primary owner, followed by the other associated fighters in roster order. Keep a separate phase for each stage/candidate sequence. A changed candidate list begins a new sequence; returning to an existing sequence resumes it.
- Retain phases within the Local or Training session, including Rematch and Reselect. Returning to the main menu clears them. A stage-cycle phase advances only when that rule selects a theme; the two-fighter phase advances only when alternation selects a theme.
- Count a started match even if a Jukebox file overrides the sound or the selected arrangement is unavailable. Cancelled setup, file availability, and resimulation do not change the sequence.
- Local and online two-player matches use the same rule when the mode supports stage selection. Until a dedicated rule is agreed, CPU-led exhibitions and World Tour encounters use the designated CPU opponent's theme; a battle with multiple CPU opponents uses its designated boss/encounter owner. Do not create a new priority based on incidental player slot order.
- Replays retain the original selected identity. Musical selection and rotation state are deterministic presentation state restored with the session/replay; they do not alter gameplay checksums. Audio output is not replayed for frames already heard during rollback.

These defaults were proposed by the author and approved by the user with the complete design on 2026-11-10.

### Jukebox and unavailable themes

Keep the existing explicit Jukebox precedence and its `NATIVE` behavior. A `NATIVE` result plays the newly selected native theme. Do not change the existing rule for competing external fighter tracks as a side effect of native-theme alternation. Make the selected native-theme event visible to the existing override resolver; the old recognition of a stage command plus the Arcade Ryu special case is insufficient.

Choose the logical theme before checking local availability. If its score is missing, invalid, or not ready, use the existing 1987 stage fallback and log the reason without preventing the fight. Preserve Ryu's current missing-pack behavior. A new stage with no native 1987 tune retains the documented `0x29` fallback. Do not silently skip unavailable members to change future rotation order.

## 5. Approaches and selected storage design

### Current constraint

The current table occupies `0x8000..0xc000`; RAM starts at `0xc000`. Tune pointers, channel positions, and relocated jumps are 16-bit. The approved Ryu probe reports **10,828 bytes used out of 16,384**, including the original test theme, leaving **5,556 bytes**. Thirty character arrangements and the test fit the available command count, but there is no evidence they fit those remaining bytes.

Increasing a capacity constant alone would overlap RAM or truncate addresses. Compressing files on disk does not increase the decompressed playback window.

### Considered approaches

1. **Small resident images selected for each fight.** Keep the present native window and store the whole library outside it. This is a smaller change, but every reachable theme set must fit. Current/saved stream lifetimes and in-fight changes require a deliberate handoff protocol; a naive table replacement can consume the new command against the old table without ever starting it. One oversized arrangement remains a problem.
2. **Larger separate storage for extended scores — selected by the user.** Keep the native CPU ROM/RAM map intact and give extended score streams a separate logical address space. This requires more converter/player work, but removes the shared 16 KiB bottleneck and supports an individual arrangement larger than that window. Current and saved playback retain their storage identity throughout interruptions.

Lossless phrase/repeat optimization remains useful, but is an implementation optimization measured against preserved timing and events, not the capacity architecture. Jukebox rendering was already excluded by the accepted native-theme design.

### Module responsibilities

- **Engine:** owns the character-to-theme registry, stage membership, match choice, and deterministic rotation state. Emits the selected theme identity with the timed music request.
- **Host loader:** discovers the local pack, validates/compiles available scores before their first playback request, reports failures, and supplies an immutable library. File reads, compilation, and source extraction do not run in the audio callback or match-selection routine.
- **Native audio player:** owns library lifetime, native versus extended stream addressing, current/saved channels, priority, gates, loops, voice controls, elapsed chip clocks, and effect handback. Callers do not manage banks or manually restore channel internals.

### Required invariants

1. Native ROM addresses retain their existing meaning. No enlargement of the sound CPU's memory map into RAM; no new work-RAM patch path in the game engine.
2. Extended positions and jump targets are checked logical positions with score/library identity. Every compiler relocation and reader must support the chosen larger address space without truncation. The old extended-pointer address-range check cannot remain the sole source discriminator.
3. Current and saved playback retain source identity, cursor, repeats, held pitch, voice controls, gates, and elapsed-clock state together. Native-to-extended and extended-to-native priority transitions, paused saved music, silently advancing saved music, restore/discard, and effect ownership preserve their existing behavior.
4. Replacing or disabling a library cannot invalidate current or saved streams. Lifetime is explicit; a pending replacement must not cause a cue to be consumed against the wrong library or lost without replay/dispatch.
5. Storage reads do not introduce disk latency into emulated time. Preserve the established elapsed-chip-clock behavior and native instruction costs. Do not promise sample-identical reproduction of a different source sound system; verify musical timing and the native writes actually produced.
6. The selected semantic identity accompanies its timestamp through `FrameOutput`, audio queues, resets, replay, and rollback deduplication. A boolean table-enable flag or a command byte alone must not ambiguously stand for different themes.
7. Faithful 1987, the attract, and the faithful cabinet load no extended library and do not open the music pack. Existing native commands retain their verified behavior.
8. Invalid scores fail independently with actionable diagnostics. A bad cursor, jump, or zero-time loop is rejected or reported; it must not hang playback or read arbitrary data.

### Compatibility and identity

Preserve the existing readable score features, including the explicit `voice`, `raw`, `pitch`, `wait`, and `off` controls. Ryu's installed score and the original test score remain valid inputs. Existing legacy native-stream tests and supported pack scores remain compatible; the builder may introduce a versioned compiled representation behind that interface.

Use stable semantic theme identities independently of filenames and runtime command allocation. Preserve Ryu's established `0x41` and the test theme's `0x40`. The older Wave 1 reservation of `0x41` for Evil Ryu must be replaced explicitly in the registry, stage metadata, affected tests, and planning references. Do not let file order decide ownership. Allocate the other commands centrally; the present `0x40..=0x7f` range has sufficient IDs for this inventory.

The builder chooses the precise cursor width/bank representation, binary layout, caches, and library APIs within these invariants. The design does not require a new player-facing storage setting.

## 6. Exact-source selection and local resources

### Source card for each arrangement

Before extracting or transcribing, record a local source card containing:

- Recipient and selected source character/cue; Western and Japanese identities where relevant.
- Game, platform, edition, region, dated revision/set, and sound version. Similar game titles or file names alone are insufficient.
- Verified archive/member hashes, required sound ROMs/SIMMs/BIOS, their local location, and the scope under which the resource is being used.
- Cue command/data address or reproducible in-game trigger; distinguish stage, rival, boss, introduction, and ending cues.
- Reference player/emulator build, capture trigger, tempo/time base, full intro, true loop boundaries, sustained/gate state at the loop, and at least two reference passes.
- Extraction tool/version and source-system profile, or an explicit account of any transcription. Source evidence and intermediate event data remain local.

**Approved default, initially proposed by the author:** use the normal fight arrangement, not a danger/urgent variant, and the project's pinned reference player where it supports the verified set. For an unpinned exact revision, choose a verified available arcade set matching the selected arrangement, document it, and identify it explicitly in the first proof. A different composition, platform arrangement, or edition requires returning to the user; a set pin is not permission to substitute one.

### What was actually found

| Family | Evidence from the read-only reconnaissance | Remaining work |
|---|---|---|
| World Warrior | Archives physically present under `the local World Warrior archive folder`; Ryu's World 910522 evidence and tools exist | Verify the selected cue/source member for each remaining fighter. The earlier explicit resource authorization was for Ryu, not an established general importer. |
| Alpha 2/3 | `the local Alpha research notes` documents previously audited arcade sets in an Emulation share; `games` and `tools` are README-only | Locate the actual current archives and verify the exact chosen sets; investigate their QSound source data. |
| 2nd Impact | `D:\Emulation\roms\cps3\sfiii2.zip` and associated CHDs exist; ZIP member metadata was inspected | Audit the exact set and music source, identify Oro's cue, and investigate CPS3 music data. No importer was verified. |
| Final Fight | No archive located in the bounded local searches | Locate a suitable authorized local original-arcade source; investigate its CPS1 driver/profile. |
| Final Fight Revenge | No game or ST-V BIOS archive located in the bounded inspected roots | Locate and verify the arcade resources; investigate ST-V music data independently. A Saturn listening copy is not an arcade source pin. |

These are availability findings, not claims that a resource is absent everywhere or that a filename establishes ownership. No extraction, copying of source ROMs, or new downloads occurred during brainstorming. Missing resources become explicit sourcing dependencies; they do not invite silent substitutions.

Ryu's CPS1 tools are a starting reference. The inspected extractor has an SF2-specific profile; Final Fight needs its own verified addresses and behavior. Alpha/QSound, CPS3, and Revenge/ST-V require separate investigation. Obtain any missing source information or authorization before dependent extraction work.

## 7. Arrangement and listening workflow

1. **Verify the source card.** Capture the selected original arrangement and inspect source data where available. Separate musical event timing from the source system's instrument and mixer behavior.
2. **Build a short proof locally.** Choose a representative passage that exposes lead, bass, harmony, register, timing, and important accompaniment. Use SF1 instruments and the intended native-player path as soon as the required capability exists. An offline renderer is an audition aid, not evidence of native integration.
3. **Review with the user.** Compare against the named source. Describe any voice merging, omitted percussion, register displacement, or hardware compromise. Resolve a material musical compromise before expanding the proof into the complete theme.
4. **Build the complete arrangement.** Preserve intro and source phrase order, make a real loop, and render at least two complete passes. A fade used to end a listening file is never encoded as the score's loop.
5. **Review individual FM parts.** Supply the complete mix and useful solo parts with names/roles and consistent reference level. Adjust voice selection, attenuation, feedback/modulation, and balance through listening.
6. **Integrate and verify.** Install approved local material beside the development ROM set and normal AppData ROM set, exercise real mode/stage selection, and verify interruption, loop, stop, fallback, and exit behavior.

Ryu's approved balance restores stock SF1 lead, bass, and lead echo while the other parts retain gentler modulation and are roughly one-third quieter. This is evidence for Ryu, not a mandatory mix preset. Preserve its approved local score and listening artifacts while developing other themes.

**Percussion:** decide at each proof review. Ryu remains FM-only. FM percussion consumes part of the same voice budget. If a chosen arrangement needs synchronized ADPCM, agree its native timing, format, stop/resume, and ownership design before implementing that capability. Do not add a host-played percussion bed automatically.

The builder may choose initial instrument mappings and representative passages, labeled as provisional. Source systems with more simultaneous voices than the target can express need an explicit reduction proposal; selecting a source does not approve an unreviewed reduction of its harmony or register.

## 8. Delivery order and acceptance evidence

**Approved order, initially proposed by the author:** first implement and verify storage/selection with original test material and the already-approved local Ryu score. Then use World Warrior as the first new source family because it is closest to the established evidence. After each family has a verified source and proof, complete its arrangements. Alpha 2/3, Final Fight, Revenge, and 2nd Impact sourcing investigations can proceed independently; their final order may follow resource readiness and the user's listening availability.

No Capcom arrangement is accepted solely because automated event assertions pass. Required evidence includes:

- Original synthetic score material demonstrating an individual score over 16 KiB and logical positions beyond the old 16-bit range, plus a combined library over 64 KiB. Exercise backward loops, jumps, validation failures, and distinguishable events across the old boundaries.
- Existing legacy score, Ryu-control, and native audio checks at the appropriate scope. Ryu's earlier 59 targeted checks and 112-second probe are historical evidence, not tests rerun by this design session.
- Native/extended priority transitions, paused and silently advancing saved themes, effect handback, stops/discard of both slots, and library replacement while saved music is alive. Check clock behavior under eight-voice load and repeated loops.
- Table-driven selection checks covering Arcade, zero/one/two associated fighters, mirrors, side swaps, multiple stages, draws, Rematch, Reselect, Training reset, cancelled setup, missing files, and Jukebox overrides.
- Replay/rollback checks proving the same logical choice and phase, and no duplicate rotation or playback during resimulation. Score availability and local library bytes must not affect gameplay outcomes/checksums.
- Real front-end playback for the selected theme, new rounds, a relevant interruption, quit/return to menu, and a subsequent Faithful 1987 fight. Reuse the existing reference sweeps for unchanged original behavior.
- Per-arrangement source evidence, short-proof approval, complete-loop approval, solo-part review, and the measured compiled size. Record checks skipped for missing local resources as skipped.

Public tests use original phrases and synthetic data. Tests relying on local Capcom material must say when they skip and must not publish that material.

## 9. Planning follow-up and existing plans to reconcile

- [ADR-0023](../adr/0023-character-themes-and-separate-native-score-storage.md) records the larger separate store and individual-theme decisions; ADR-0009 points to this amendment while retaining its native playback and Faithful-isolation commitments.
- Wave issues [#31](https://github.com/johnstreetfighter/sf1-port/issues/31), [#49](https://github.com/johnstreetfighter/sf1-port/issues/49), and [#62](https://github.com/johnstreetfighter/sf1-port/issues/62) describe original shared-stage themes and older command assignments. Reconcile them with individual character themes, selected local arrangements, and the new resolution rule. This design does not create extra original compositions to satisfy stale wording implicitly.
- Stage membership from ADR-0020/0021 remains useful. The older one-stage/one-theme-per-challenger language is superseded for the shared stages covered here.
- Update domain vocabulary, source/score-format references, and affected issue acceptance criteria through the approved planning process. Preserve the distinction between a native theme and a Jukebox track.

The approved planning follow-up updates the domain vocabulary and records the architectural decision. Existing wave issues will be reconciled through the reviewed ticket breakdown; production code and music assets remain unchanged during this planning pass.
