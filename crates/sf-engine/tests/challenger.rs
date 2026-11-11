use std::collections::BTreeMap;
use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::fight::FightMachine;
use sf_engine::run::{compare_fight_ram_with_inputs, observe_cpu_fight, CpuRamOutcome};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const SCENARIO: &str = "../../tools/reference/scenarios/sf_challenger_01.json";
const RUN_A: &str = "../../tools/reference/local/captures/challenger-01-run-a";
const RUN_B: &str = "../../tools/reference/local/captures/challenger-01-run-b";
const CPU_01_A: &str = "../../tools/reference/local/captures/cpu-01-run-a";

fn present() -> bool {
    [ROMS, MANIFEST, SCENARIO, RUN_A, RUN_B, CPU_01_A]
        .iter()
        .all(|p| Path::new(p).exists())
}

#[test]
fn sf_challenger_01_reference_runs_repeat_exactly() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let scenario = load_scenario(SCENARIO).unwrap();
    assert_eq!(scenario.checkpoints, (2389..=5530).collect::<Vec<_>>());
    let scope = CompareScope {
        from: Some(2389),
        to: Some(5530),
        fields: None,
    };
    let outcome = compare(
        &scenario,
        &load_capture(Path::new(RUN_A)).unwrap(),
        &load_capture(Path::new(RUN_B)).unwrap(),
        CandidateKind::Reference,
        &scope,
    )
    .unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared: 3142,
                checkpoints_compared: 3142
            }
        ),
        "{outcome:?}"
    );
}

#[test]
fn challenger_01_frame_2389_differences_against_cpu_01_are_exhaustively_listed() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let chal_vid = std::fs::read(Path::new(RUN_A).join("ram_002389_video.bin")).unwrap();
    let cpu_vid = std::fs::read(Path::new(CPU_01_A).join("ram_002389_video.bin")).unwrap();
    assert_eq!(chal_vid, cpu_vid, "video RAM at 2389 must be identical");

    let chal_pal = std::fs::read(Path::new(RUN_A).join("ram_002389_palette.bin")).unwrap();
    let cpu_pal = std::fs::read(Path::new(CPU_01_A).join("ram_002389_palette.bin")).unwrap();
    assert_eq!(chal_pal, cpu_pal, "palette RAM at 2389 must be identical");

    let chal_work = std::fs::read(Path::new(RUN_A).join("ram_002389_work.bin")).unwrap();
    let cpu_work = std::fs::read(Path::new(CPU_01_A).join("ram_002389_work.bin")).unwrap();
    assert_eq!(chal_work.len(), cpu_work.len());

    let mut diffs = Vec::new();
    for i in 0..chal_work.len() {
        if chal_work[i] != cpu_work[i] {
            diffs.push((0xff8000 + i as u32, chal_work[i], cpu_work[i]));
        }
    }

    // List and classify every one of the 9 byte differences between sf-challenger-01 and sf-cpu-01 at 2389:
    // 1. 0xff8001 (CURRENT_SLOT): 0x07 vs 0x03 (slot 7's 7-frame cycle was shifted by 6 frames
    //    during coin2's counter pulse at 0x4833e, so slot 7 ran in frame 2389).
    // 2. 0xff85e1..0xff8622 (slot 7's 68000 stack frame in 0xff85c0..0xff8622):
    //    - 0xff85e1 (usp + 3, saved D0.b sleep count): 0x01 vs 0x06
    //    - 0xff85f9 (usp + 27, saved D6.b coin-counter output image): 0x72 vs 0x71
    //    - 0xff8605 (usp + 39, saved A1 low byte): 0x8f (0xff868f) vs 0x5a (0xff865a)
    //    - 0xff8620..0xff8622 (usp + 66..68, return PC low word): 0x819e (0x04819e) vs 0x8270 (0x048270)
    // 3. 0xff865d (CREDITS + 1): 0x01 vs 0x00 (1 credit remaining from coin2 at 1480).
    // 4. 0xff865f and 0xff8661 (sound queue read/write indices): 0x04 vs 0x02 (advanced by 1 word
    //    from coin2's sound 0x16, already consumed and cleared back to 0xffff by vblank).
    assert_eq!(
        diffs,
        vec![
            (0xff8001, 0x07, 0x03),
            (0xff85e1, 0x01, 0x06),
            (0xff85f9, 0x72, 0x71),
            (0xff8605, 0x8f, 0x5a),
            (0xff8620, 0x81, 0x82),
            (0xff8621, 0x9e, 0x70),
            (0xff865d, 0x01, 0x00),
            (0xff865f, 0x04, 0x02),
            (0xff8661, 0x04, 0x02),
        ]
    );
}

#[test]
fn native_engine_matches_challenger_01_from_2389_through_versus_first_round_result() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();
    let outcome = compare_fight_ram_with_inputs(
        Path::new(RUN_A),
        rom.clone(),
        Some(Path::new(SCENARIO)),
        2389,
        5530,
    )
    .unwrap();
    assert_eq!(outcome, CpuRamOutcome::Match { frames: 3141 });

    let landmarks = [
        2390, 3102, 3136, 3205, 3343, 3723, 3750, 3751, 3752, 3871, 3872, 3967, 3968, 4207, 4208,
        4306, 4307, 4458, 4459, 4627, 4739, 4938, 4970, 5522, 5523, 5530,
    ];
    let mut snapshots = BTreeMap::new();
    let m0 = FightMachine::from_fight_checkpoint(Path::new(RUN_A), 2389, rom.clone()).unwrap();
    snapshots.insert(2389, m0.work_ram().to_vec());
    let mut video_3752 = Vec::new();
    let mut video_3967 = Vec::new();
    let mut latches = Vec::new();
    let (last, unrecovered) = observe_cpu_fight(
        Path::new(RUN_A),
        rom,
        Some(Path::new(SCENARIO)),
        2389,
        5530,
        |m| {
            let f = m.state.frame;
            if landmarks.contains(&f) {
                snapshots.insert(f, m.work_ram().to_vec());
            }
            if f == 3752 {
                video_3752 = m.state.videoram.clone();
            }
            if f == 3967 {
                video_3967 = m.state.videoram.clone();
            }
            if (3750..=3970).contains(&f) && m.state.sound_latch != 0xff {
                latches.push((f, m.state.sound_latch));
            }
        },
    )
    .unwrap();
    assert_eq!((last, unrecovered), (5530, vec![]));
    let w = |frame| &snapshots[&frame][..];
    let rl = |frame, off: usize| -> u32 {
        let s = w(frame);
        u32::from_be_bytes([s[off], s[off + 1], s[off + 2], s[off + 3]])
    };
    let rw = |frame, off: usize| -> u16 {
        let s = w(frame);
        u16::from_be_bytes([s[off], s[off + 1]])
    };

    // 1. At 2389 -> 2390: 1 credit remains at 0xff865d; Round 1 starts at 2390 and slot 3 arms at 0x3f47e.
    assert_eq!(
        (
            w(2389)[0x4002],
            w(2389)[0x4008],
            w(2389)[0x65d],
            rl(2389, 0xf24)
        ),
        (2, 0x00, 1, 0x3f41e)
    );
    assert_eq!(
        (
            w(2390)[0x4002],
            w(2390)[0x4008],
            w(2390)[0x65d],
            rl(2390, 0xf24)
        ),
        (2, 0x80, 1, 0x3f47e)
    );

    // 2. Round 1 ends at 3102; during Round 1 result settle (3136..=3342), start2 at 3200..3212
    //    is ignored because 0xffc008 == 0x20 != 0x80 (slot 3 stays at 0x3f41e, credits stay 1).
    assert_eq!(
        (w(3102)[0x4008], rl(3102, 0xf24), w(3102)[0x65d]),
        (0x00, 0x3f41e, 1)
    );
    assert_eq!(
        (
            w(3205)[0x4008],
            w(3205)[0xe36],
            rl(3205, 0xf24),
            w(3205)[0x65d]
        ),
        (0x20, 7, 0x3f41e, 1)
    );

    // 3. Round 2 starts at 3723 (0xffc008 == 0x80, ROUND_NO == 1, slot 3 armed at 0x3f47e).
    assert_eq!(
        (
            w(3723)[0x4002],
            w(3723)[0x4008],
            w(3723)[0x400a],
            rl(3723, 0xf24),
            w(3723)[0x65d]
        ),
        (2, 0x80, 1, 0x3f47e, 1)
    );

    // 4. At 3751 (start2 held from 3750): slot 3 passes 0x3f4a2 (buyin 3 > stage 0), sets
    //    0xffc008 = 0x10 and 0xff8f24 = 0x3f4c6, and takes 1 credit (0xff865d = 0).
    assert_eq!(
        (w(3750)[0x4008], rl(3750, 0xf24), w(3750)[0x65d]),
        (0x80, 0x3f47e, 1)
    );
    assert_eq!(
        (w(3751)[0x4008], rl(3751, 0xf24), w(3751)[0x65d]),
        (0x10, 0x3f4c6, 0)
    );

    // 5. At 3752: fight_frame enters 0x423b8, queues 0x3f and 0x3e, draws 0x15ba2
    //    (bordered challenger frame starting at 0x052c = 0x90c4), holds 120 frames (3752..=3871),
    //    types 0x15c30 (HERE COMES A NEW CHALLENGER starting at 0x0636 = 0x9011) at D4 = 3 over
    //    3872..=3967, holds 240 frames (3968..=4207), and wipes via event 6 (4208..=4306).
    assert_eq!(
        u16::from_be_bytes([video_3752[0x052c], video_3752[0x052d]]),
        0x90c4,
        "0x15ba2 frame drawn at 3752"
    );
    assert_eq!(
        u16::from_be_bytes([video_3752[0x0636], video_3752[0x0637]]),
        0x103c,
        "0x15c30 blank inside frame at 3752"
    );
    assert_eq!(
        u16::from_be_bytes([video_3967[0x0636], video_3967[0x0637]]),
        0x9011,
        "0x15c30 'H' typed by 3967"
    );
    assert!(
        latches.contains(&(3753, 0x3f)) && latches.contains(&(3754, 0x3e)),
        "challenger latches: {latches:02x?}"
    );
    assert_eq!(
        (w(4208)[0x4008], w(4208)[0xe36], rl(4208, 0xf24)),
        (0x30, 6, 0x3f4c6)
    );
    assert_eq!(
        (w(4306)[0x4008], w(4306)[0xe36], rl(4306, 0xf24)),
        (0x10, 6, 0x3f4c6)
    );

    // 6. At 4307: 0x423fc..0x4241c sets GAME_MODE = 1, ROUND_NO = 0, SCENE_MODE = 0,
    //    clears objects, places both human players via 0x3e5ea (P1 at 0x0400, P2 at 0x0a00),
    //    and requests event 4 in its versus shape (0x3f9fc: bg_scroll 0x780, palette 0x1ae68),
    //    while slot 3 at 0x3f4c6 sees SCENE_MODE & 0x10 == 0 and resets 0xff8f24 = 0x3f41e.
    assert_eq!(
        (
            w(4307)[0x4002],
            w(4307)[0x4008],
            w(4307)[0x400a],
            w(4307)[0xe36],
            rl(4307, 0xf24)
        ),
        (1, 0x20, 0, 4, 0x3f41e)
    );
    assert_eq!(
        (
            rw(4307, 0x6ba),
            rw(4307, 0x6bc),
            rw(4307, 0x6be),
            rw(4307, 0x6c0),
            rw(4307, 0x6d0)
        ),
        (0x02bc, 0x0298, 0x0a00, 0x0400, 0x019a)
    );
    assert_eq!(
        (
            rw(4307, 0x6e4),
            rw(4307, 0x6e6),
            rw(4307, 0x6e8),
            rw(4307, 0x6ea),
            rw(4307, 0x6fa)
        ),
        (0x03bc, 0x02e0, 0x0400, 0x0a00, 0x01dc)
    );
    assert_eq!(
        (rw(4307, 0x4682), rl(4307, 0x634), rw(4307, 0x638)),
        (0x0780, 0x1ae68, 0)
    );

    // 7. Event 5 (ROUND 1) runs over 4459..=4627; versus Round 1 starts at 4739;
    //    P1 Ryu KOs P2 Ken at 4938; first versus round result runs 4970..=5522 (P1 wins = 1).
    assert_eq!(
        (w(4459)[0x4002], w(4459)[0x4008], w(4459)[0xe36]),
        (1, 0x20, 5)
    );
    assert_eq!(
        (w(4739)[0x4002], w(4739)[0x4008], w(4739)[0x400a]),
        (1, 0x80, 0)
    );
    assert_eq!((w(4938)[0x4002], w(4938)[0x4008]), (1, 0x00));
    assert_eq!(
        (
            w(4970)[0x4002],
            w(4970)[0x4008],
            w(4970)[0xe36],
            w(4970)[0x6b8],
            w(4970)[0x6e2]
        ),
        (1, 0x20, 7, 1, 0)
    );
    assert_eq!(
        (w(5522)[0x4002], w(5522)[0x4008], w(5522)[0x6b8]),
        (1, 0x00, 1)
    );
    assert_eq!(
        (
            w(5523)[0x4002],
            w(5523)[0x4008],
            w(5523)[0x400a],
            w(5523)[0xe36]
        ),
        (1, 0x20, 1, 6)
    );
}

#[test]
fn buyin_limit_at_0x3f4a2_refuses_challenger_join_when_buyin_le_stage_and_mode_4_watches_start1() {
    if !present() {
        eprintln!("skipping: local captures/ROMs not present");
        return;
    }
    let rom = sf_video::romset::load_program_rom(ROMS, MANIFEST).unwrap();

    // 1. C-NEG on 0x3f4a2..0x3f4aa (`move.b ($4001,A5),D0; cmp.b ($4003,A5),D0; ble $3f4d2`):
    //    when buyin <= stage (both equal and strictly less), pressing start2 at 3750 is refused,
    //    resets 0xff8f24 = 0x3f41e, leaves SCENE_MODE == 0x80 and credits == 1, and continues the fight.
    for (buyin, stage) in [(0u8, 0u8), (2, 2), (1, 2)] {
        let mut m =
            FightMachine::from_fight_checkpoint(Path::new(RUN_A), 3750, rom.clone()).unwrap();
        m.set_work_byte(0xffc001, buyin);
        m.set_work_byte(0xffc003, stage);
        m.set_cabinet_inputs(false, false, false, true, false); // start2 held
        m.step();
        assert!(m.unrecovered().is_empty());
        let w = m.work_ram();
        let watch = u32::from_be_bytes([w[0xf24], w[0xf25], w[0xf26], w[0xf27]]);
        assert_eq!(
            (w[0x4008], w[0x65d], watch),
            (0x80, 1, 0x3f41e),
            "buyin={buyin} <= stage={stage} must refuse join and reset watch to 0x3f41e"
        );
    }

    // 2. When buyin > stage (e.g. buyin = 2, stage = 1), pressing start2 is accepted.
    {
        let mut m =
            FightMachine::from_fight_checkpoint(Path::new(RUN_A), 3750, rom.clone()).unwrap();
        m.set_work_byte(0xffc001, 2);
        m.set_work_byte(0xffc003, 1);
        m.set_cabinet_inputs(false, false, false, true, false);
        m.step();
        assert!(m.unrecovered().is_empty());
        let w = m.work_ram();
        let watch = u32::from_be_bytes([w[0xf24], w[0xf25], w[0xf26], w[0xf27]]);
        assert_eq!((w[0x4008], w[0x65d], watch), (0x10, 0, 0x3f4c6));
    }

    // 3. In Mode 4 (Ken's 1P game), slot 3 arms D2 = 1 (start1) at 0x3f41e..0x3f454,
    //    ignores start2, and accepts start1 when buyin > stage.
    {
        let mut m = FightMachine::from_fight_checkpoint(Path::new(RUN_A), 2389, rom).unwrap();
        m.set_work_byte(0xffc002, 4); // GAME_MODE = 4
                                      // Frame 2390: round_start sets SCENE_MODE = 0x80; watch advances 0x3f41e -> 0x3f454 -> 0x3f47e with D2 = 1.
        m.step();
        let w = m.work_ram();
        assert_eq!(
            (
                w[0x4008],
                u32::from_be_bytes([w[0xf24], w[0xf25], w[0xf26], w[0xf27]])
            ),
            (0x80, 0x3f47e)
        );
        // Frame 2391: start2 is ignored in Mode 4.
        m.set_cabinet_inputs(false, false, false, true, false);
        m.step();
        let w = m.work_ram();
        assert_eq!((w[0x4008], w[0x65d]), (0x80, 1));
        // Frame 2392: start1 joins the challenger in Mode 4.
        m.set_cabinet_inputs(false, false, true, false, false);
        m.step();
        let w = m.work_ram();
        assert_eq!(
            (
                w[0x4008],
                w[0x65d],
                u32::from_be_bytes([w[0xf24], w[0xf25], w[0xf26], w[0xf27]])
            ),
            (0x10, 0, 0x3f4c6)
        );
    }
}
