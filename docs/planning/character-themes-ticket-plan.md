# Character themes: approved ticket breakdown

- **Parent spec:** [#75 — Individual character themes and larger native score storage](https://github.com/johnstreetfighter/sf1-port/issues/75)
- **Design:** [Approved character-theme design](../design/2026-11-10-character-themes-design.md)
- **Status:** Approved by the user and published on 2026-11-10: forty native sub-issues and 115 verified blocking edges. See the [published issue index](character-themes-issues.md).
- **Count:** 40 child tickets: five engineering slices, five source-family investigations/first proofs, twenty-nine individual arrangements, and one final acceptance ticket.

The numbers below are design ticket identifiers, not GitHub issue numbers; the published issue index maps each one to its issue and native blockers. All forty issues were created in dependency order and attached to the parent spec. The parent body, title, labels, and open status were verified unchanged.

## Common delivery rules

- A technical ticket lands a narrow, verifiable path through the existing game/host/native-audio seams. It must preserve existing supported behavior; tests are scoped to the behavior changed, not private representation.
- The first engineering slice expands the identity/library interface beside legacy inputs so existing callers remain valid. Subsequent slices land green; basic saved-state correctness belongs with new cursor/storage support, not with a knowingly broken intermediate player.
- Source-family investigations can begin independently of the storage work. A short proof can use the existing bounded native audition path; a source-specific importer is not assumed to exist. If obtaining a first verified proof uncovers work too large for the ticket, return with evidence and a proposed split rather than silently expanding the task.
- Prefer verified source music data when available; documented transcription remains an allowed path. A family investigation does not require reverse engineering an extractor when a reliable, reviewed transcription can satisfy the approved arrangement and provenance requirements.
- A source-family investigation finishes only with the exact source card, reproducible source cue/loop evidence, a verified family-specific path, and a short native proof approved by the user. Unavailable resources remain an explicit dependency; no placeholder or console substitution satisfies the ticket.
- Each arrangement ticket contains source verification, a user-approved short proof before full expansion, complete mix and individual FM parts reviewed by the user, measured size, and approved native installation/playback evidence. The family-first recipient reuses the first proof instead of recreating it.
- Listening checkpoints are genuine gates. Automated checks do not approve musical balance or authorize expansion beyond an unapproved proof. An agent can prepare evidence and revisions; it cannot infer approval or close the arrangement autonomously.
- All Capcom scores, source events, proof/full/stem audio, and resource-specific evidence remain local. Public commits contain reusable tooling, metadata, documentation, and original synthetic tests.
- Preserve the accepted Ryu score and balance. There is no new Ryu arrangement ticket.
- Fixtures may exercise planned shared-stage groups without releasing a wave. Actual appearances in unfinished modes are not prerequisites for source preparation or native audition; production roster gates remain intact.

## Approved triage

- Engineering tickets **1–5:** `ready-for-agent`, subject to their native blocking edges.
- Source investigations **6–10**, arrangements **11–39**, and final acceptance **40:** `ready-for-human` because their completion includes interactive listening or source review. Agents may prepare and verify the material, but a human must complete the stated acceptance checkpoint.
- If a required source location, scope of use, or exact resource cannot be resolved by investigation, use `needs-info` and state the missing prerequisite. A historical audit or an unrelated listening recording does not satisfy it.

The user approved these review-driven labels as an explicit exception to the ticket skill's usual `ready-for-agent` default. They were applied and verified on all forty published issues.

## Engineering slices

1. **Carry stable theme identity through existing Ryu playback.**
   - **Blocked by:** None.
   - **What it delivers:** The approved Ryu theme and original test material run from semantic match requests through timed audio delivery while their existing readable scores remain valid. The public registry covers the selected assignments, preserves Ryu's `0x41` and test `0x40`, and explicitly resolves the old Evil Ryu collision. Expand the new identity/library interface beside legacy inputs; do not change Ryu's arrangement or release roster content.

2. **Play oversized native scores from a larger separate library.**
   - **Blocked by:** 1.
   - **What it delivers:** Original synthetic material plays in a real overhaul fight from an individual score over 16 KiB and a library over 64 KiB, exercising positions beyond the old 16-bit range, loops, and jumps without changing the native ROM/RAM map. Existing scores remain valid, basic interruption/save/restore state is correct, and invalid positions/jumps fail independently rather than hanging playback.

3. **Retain interrupted themes and pending cues through library changes.**
   - **Blocked by:** 2.
   - **What it delivers:** Oversized current and saved themes retain source, cursor, voice, gate, loop, and clock state across mixed native/extended priorities, paused and silent advancement, effect handback, stops/discards, replacement, and disabling. A requested theme is never consumed against the wrong library or lost during deferred replacement. Verify repeated loops under eight-voice load and a subsequent Faithful fight.

4. **Select and rotate character themes in real matches.**
   - **Blocked by:** 1.
   - **What it delivers:** Arcade follows the CPU opponent; Local/Training implement the zero/one/two-associated-fighter rule and approved sequence defaults. Live current-roster matches and shared-stage fixtures cover mirrors, side swaps, rounds, cancelled setup, Rematch, Reselect, Training resets, Jukebox NATIVE/overrides, and missing-score fallback without changing release gates. Reconcile the older shared-stage/command planning references affected by this registry and policy.

5. **Preserve theme choices through replay and rollback.**
   - **Blocked by:** 4.
   - **What it delivers:** Selected identity, timestamp, and sequence phase survive session resets, replay, snapshots, and supported online rollback. Resimulation neither advances rotation twice nor replays already-heard audio. Missing local music and library bytes do not change gameplay outcomes or checksums.

## Independent source-family investigations and first proofs

6. **Verify Alpha 2 and review Dan's first native proof.**
   - **Blocked by:** None; locating and auditing the actual arcade resources is a required investigation step.
   - **What it delivers:** An exact original-arcade Alpha 2 source card and Dan cue/loop evidence, a verified QSound-specific extraction or transcription path, and an approved short SF1-instrument native proof. Establish reusable evidence/tooling for the other four selected Alpha 2 arrangements without assuming Alpha 1, Zero 2 Alpha, Gold, or console equivalence.

7. **Verify Alpha 3 and review Evil Ryu's first native proof.**
   - **Blocked by:** None; locating and auditing the actual arcade resources is a required investigation step.
   - **What it delivers:** An exact arcade source for Black Power, reproducible cue/loop evidence, a source-specific path, and an approved short native proof. Verify this game's profile independently of Alpha 2; identify the selected music even if a release's playable Evil Ryu availability differs.

8. **Verify Final Fight CPS1 and review Guy's Slum 1 proof.**
   - **Blocked by:** None; locating and auditing the original arcade resources is required.
   - **What it delivers:** Exact 1989 arcade source and Slum 1 evidence, a verified extraction profile or documented transcription, and an approved short native proof. Establish the shared source groundwork for all eight selected original Final Fight cues without inheriting SF2-specific addresses or substituting Final Fight CD.

9. **Verify arcade Revenge and review Damnd's first native proof.**
   - **Blocked by:** None; locating and auditing the actual game and required ST-V resources is required.
   - **What it delivers:** An exact arcade source card, the DAMND cue/loop evidence, a verified ST-V-specific music path, and an approved short native proof. A Saturn recording is not an arcade-source pin. Establish the groundwork needed for Belger's separate selected cue.

10. **Verify 2nd Impact and review Oro's first native proof.**
    - **Blocked by:** None; the located resources still require exact-set and cue verification.
    - **What it delivers:** Identify Oro's normal music as actually played in original arcade 2nd Impact, record its exact cue/data association and loop, verify the CPS3-specific path, and obtain approval of a short native proof. Do not invent a title or substitute another SFIII entry.

## Individual World Warrior arrangements

These use the existing World Warrior source lineage as the starting point. Each still verifies its own cue and event data. The selected reference matches Ryu's original 1991 World 910522 arrangement; other SFII revisions and console arrangements are not substitutes.

11. **Arrange Ken's original World Warrior theme.**
    - **Blocked by:** 3, 4.
    - **What it delivers:** Ken's own approved proof, complete arrangement, mix and FM parts, measured size, and native installation using the common acceptance contract. This is the first new World Warrior arrangement.

12. **Arrange Chun-Li's original World Warrior theme.**
    - **Blocked by:** 3, 4.
    - **What it delivers:** Chun-Li's selected 1991 theme through the complete proof/review/native-installation contract, with her own instrument and balance decisions.

13. **Arrange Guile's original World Warrior theme.**
    - **Blocked by:** 3, 4.
    - **What it delivers:** Guile's selected 1991 theme through the complete proof/review/native-installation contract.

14. **Arrange Dhalsim's original World Warrior theme.**
    - **Blocked by:** 3, 4.
    - **What it delivers:** Dhalsim's selected 1991 theme through the complete proof/review/native-installation contract, making necessary voice/percussion decisions explicit.

15. **Arrange Zangief's original World Warrior theme.**
    - **Blocked by:** 3, 4.
    - **What it delivers:** Zangief's selected 1991 theme through the complete proof/review/native-installation contract.

16. **Arrange Vega's original World Warrior theme.**
    - **Blocked by:** 3, 4.
    - **What it delivers:** The claw fighter's selected 1991 theme through the complete proof/review/native-installation contract, with Western/Japanese source identities recorded explicitly.

17. **Arrange Balrog's original World Warrior theme.**
    - **Blocked by:** 3, 4.
    - **What it delivers:** The boxer's selected 1991 theme through the complete proof/review/native-installation contract, with Western/Japanese source identities recorded explicitly.

18. **Arrange M. Bison's original World Warrior theme.**
    - **Blocked by:** 3, 4.
    - **What it delivers:** The dictator's selected 1991 theme through the complete proof/review/native-installation contract, with Western/Japanese source identities recorded explicitly.

## Individual Alpha 2 arrangements

19. **Complete Dan's arcade Alpha 2 arrangement.**
    - **Blocked by:** 3, 4, 6.
    - **What it delivers:** Expand the approved family-first Dan proof into the complete mix and individual FM parts, secure the remaining listening approvals, and install/verify the native score without redoing the accepted proof.

20. **Arrange Akuma's arcade Alpha 2 stage theme.**
    - **Blocked by:** 3, 4, 6.
    - **What it delivers:** Akuma's selected Alpha 2 cue through his own source card, approved proof, complete mix/parts, and native installation. Do not substitute a Shin or Alpha 3 cue.

21. **Arrange Rose's arcade Alpha 2 stage theme.**
    - **Blocked by:** 3, 4, 6.
    - **What it delivers:** Rose's selected Alpha 2 cue through the complete proof/review/native-installation contract.

22. **Arrange Charlie Nash's arcade Alpha 2 stage theme.**
    - **Blocked by:** 3, 4, 6.
    - **What it delivers:** Charlie/Nash's selected Alpha 2 cue through the complete proof/review/native-installation contract.

23. **Arrange Gen's arcade Alpha 2 theme for Dorai.**
    - **Blocked by:** 3, 4, 6.
    - **What it delivers:** The approved borrowed Gen cue assigned to Dorai through the complete proof/review/native-installation contract. Original Gen retains his native SF1 theme.

## Individual Alpha 3 arrangements

24. **Complete Black Power for Evil Ryu.**
    - **Blocked by:** 3, 4, 7.
    - **What it delivers:** Expand the approved Black Power proof, review the complete mix and FM parts, and install/verify Evil Ryu's native score using his reconciled identity/command. Preserve regular Ryu's score and balance.

25. **Arrange The Road for Gouken.**
    - **Blocked by:** 3, 4, 7.
    - **What it delivers:** Ryu's selected arcade Alpha 3 composition assigned to Gouken through the complete proof/review/native-installation contract.

26. **Arrange Feel the Cool for Goutetsu.**
    - **Blocked by:** 3, 4, 7.
    - **What it delivers:** Akuma's regular arcade Alpha 3 composition assigned to Goutetsu through the complete contract; do not substitute Groan.

27. **Arrange Performance for Gou Hibiki.**
    - **Blocked by:** 3, 4, 7.
    - **What it delivers:** Dan's selected arcade Alpha 3 composition assigned to his father through the complete proof/review/native-installation contract.

28. **Arrange No More Swingin' for Zeku.**
    - **Blocked by:** 3, 4, 7.
    - **What it delivers:** Guy's selected arcade Alpha 3 composition assigned to his teacher through the complete proof/review/native-installation contract.

## Individual original Final Fight arrangements

29. **Complete Slum 1 for Guy.**
    - **Blocked by:** 3, 4, 8.
    - **What it delivers:** Expand the approved original-arcade Slum 1 proof into the complete mix/parts and accepted native score. Preserve the published cue's intro and real loop rather than an audition fade.

30. **Arrange Subway Park 1 for Cody.**
    - **Blocked by:** 3, 4, 8.
    - **What it delivers:** The selected original arcade cue through Cody's complete proof/review/native-installation contract; this is distinct from Sodom's Subway Park 2.

31. **Arrange Slum 2 for Haggar.**
    - **Blocked by:** 3, 4, 8.
    - **What it delivers:** The selected original arcade cue through Haggar's complete proof/review/native-installation contract; this is distinct from Guy's Slum 1.

32. **Arrange Subway Park 2 for Sodom.**
    - **Blocked by:** 3, 4, 8.
    - **What it delivers:** The selected original arcade ring-fight cue through Sodom's complete proof/review/native-installation contract.

33. **Arrange Industrial Area 2 for Rolento.**
    - **Blocked by:** 3, 4, 8.
    - **What it delivers:** The selected original arcade cue through Rolento's complete proof/review/native-installation contract; do not substitute Industrial Area 1.

34. **Arrange West Side 1 for Hugo.**
    - **Blocked by:** 3, 4, 8.
    - **What it delivers:** The approved Andore-associated original arcade cue through Hugo's complete proof/review/native-installation contract.

35. **Arrange Bay Area 3 for Abigail.**
    - **Blocked by:** 3, 4, 8.
    - **What it delivers:** The selected original arcade cue through Abigail's complete proof/review/native-installation contract; preserve its identity independently of Belger's different selected Revenge theme.

36. **Arrange Bay Area 1 for Poison.**
    - **Blocked by:** 3, 4, 8.
    - **What it delivers:** The selected original arcade stage cue deliberately assigned to Poison through the complete proof/review/native-installation contract.

## Individual arcade Revenge arrangements

37. **Complete DAMND for Damnd.**
    - **Blocked by:** 3, 4, 9.
    - **What it delivers:** Expand the approved arcade Revenge proof into the complete mix/parts and accepted native score. Keep the exact arcade source identity visible through review.

38. **Arrange BELGER for Belger.**
    - **Blocked by:** 3, 4, 9.
    - **What it delivers:** The selected arcade Revenge cue through Belger's own approved proof, full mix/parts, measured size, and native installation.

## Individual 2nd Impact arrangement

39. **Complete Oro's arcade 2nd Impact arrangement.**
    - **Blocked by:** 3, 4, 10.
    - **What it delivers:** Expand the approved, exactly identified Oro proof into the complete mix and individual FM parts, secure musical acceptance, and install/verify native playback without substituting another SFIII game's cue.

## Final acceptance

40. **Accept and install the complete local character-theme pack.**
    - **Blocked by:** 3, 4, 5, and every arrangement ticket 11–39.
    - **What it delivers:** The approved Ryu arrangement plus all twenty-nine accepted new scores form the installed thirty-theme library beside both development and normal AppData ROM sets. Verify the ten retained SF1 opponents, identity/rotation/fallback/Jukebox behavior, a representative real match per available source family/context, native interruptions and exits, replay/rollback, and subsequent Faithful play. Record all listening approvals, sizes, hashes, passed checks, and resource/release-dependent skips. Keep Capcom material out of public commits; do not release an unfinished roster wave or rewrite the accepted Ryu music.

## Dependency and granularity notes

- Tickets 1, 6, 7, 8, 9, and 10 form the initial investigation/engineering frontier. A known missing resource can halt the dependent proof, but does not artificially block unrelated source work.
- Tickets 2 and 4 can proceed after 1; ticket 5 follows 4, while ticket 3 follows 2. Arrangements require the complete native storage/lifetime path and the selection path, plus their own family evidence where new.
- Ken is scheduled as the first new World Warrior arrangement because that is the approved delivery preference. The other World Warrior arrangements do not have a false dependency on Ken's subjective acceptance.
- Alpha 2 and Alpha 3 are independent source investigations. Neither is blocked merely because the other uses QSound. Final Fight does not inherit an SF2 address profile; Revenge does not inherit Final Fight's sound system.
- Replay is a prerequisite for complete-pack acceptance, not for composing every individual theme. A later arrangement that exposes a transport defect can require a targeted follow-up rather than forcing every source task behind every transport test.
- Twenty-nine per-character tickets keep independent musical review, revisions, provenance, and acceptance visible. The five first proofs are reused in their recipient's completion ticket. This is forty tickets, not forty implementation runs without user interaction.
