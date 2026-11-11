//! Ticket #138 (`F66`): a two-player session from power-on (`sf-full-2p-01`):
//!
//! Power-on -> warning -> attract wall -> title -> attract ranking -> attract
//! demo fight -> first coin during demo (`0xff8026 == 0x02` occupied slot 2
//! killed at `0x4823e`) -> second coin -> 2P start -> 2P country select ->
//! versus match #1 won 2-0 by P2 Ken (`< 5,000` pts, `0x3eebc` checks P1 Ryu,
//! P2 Ken enters `GAME_MODE = 4` against Retsu) -> mid-game coin -> P1 joins
//! P2's mode-4 fight as challenger via `0x3f448` (`D2 = 1` -> `0x423b8`
//! `HERE COMES A NEW CHALLENGER`) -> versus match #2 won 2-0 by P1 Ryu
//! (`< 5,000` pts, `0x3ee8a` checks P2 Ken and clears 4,880 to 0, P1 Ryu
//! enters `GAME_MODE = 2` against Retsu) -> mid-game coin -> P1 loses 0-2 to
//! Retsu (`0x3ef18` clears P1's 4,880 score to 0 and enters continue) -> P1
//! continues via `start1` (`0x3f146`) -> P1 loses 0-2 to Retsu again ->
//! continue countdown expires (`0x3f1bc`) -> game over (`0x3f1bc..0x3f274`,
//! event 10, `0x3f33c..0x3f372`) -> return to attract (`0x3ccae` at `21213`)
//! through `21299`.

use std::collections::BTreeSet;
use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::run::{
    cpu_owned_addresses, observe_power_on_scenario, run_power_on_scenario_capture_for_reference,
    ScratchDir, RESET_CLEAR_FRAME0_END,
};
use sf_video::romset::load_assets;

const FULL_2P_SCENARIO: &str = "../../tools/reference/scenarios/sf_full_2p_01.json";
const FULL_2P_A: &str = "../../tools/reference/local/captures/full-2p-01-run-a";
const FULL_2P_B: &str = "../../tools/reference/local/captures/full-2p-01-run-b";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn present() -> bool {
    [FULL_2P_A, FULL_2P_B, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn assert_runs_identical_with_count(run_a: &str, run_b: &str, expected_files: usize) {
    let entries_a: BTreeSet<_> = std::fs::read_dir(run_a)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    let entries_b: BTreeSet<_> = std::fs::read_dir(run_b)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries_a, entries_b);
    let mut compared = 0usize;
    for name in &entries_a {
        let s = name.to_string_lossy();
        if s == "generated_driver.lua" || s == "summary.json" {
            continue;
        }
        let pa = Path::new(run_a).join(name);
        if !pa.is_file() {
            continue;
        }
        assert_eq!(
            std::fs::read(&pa).unwrap(),
            std::fs::read(Path::new(run_b).join(name)).unwrap(),
            "run-a vs run-b mismatch in {s}"
        );
        compared += 1;
    }
    assert_eq!(compared, expected_files);
}

#[test]
fn full_2p_session_from_power_on_matches_reference_and_catches_1_frame_input_shift() {
    if !present() {
        eprintln!("skipping: local sf-full-2p-01 captures/ROMs not present");
        return;
    }

    let assets = load_assets(ROMS, MANIFEST).unwrap();
    let scen = load_scenario(FULL_2P_SCENARIO).unwrap();
    assert_eq!(scen.duration_frames, 21300);
    assert_eq!(scen.checkpoints.len(), 2057);
    assert_eq!((scen.dsw1, scen.dsw2), (0xdfff, 0xffff));

    // 1. C-REF: full-2p-01-run-a == full-2p-01-run-b across all 12,345 observation files
    //    (2,057 checkpoints * 6 files + 3 TSVs).
    assert_runs_identical_with_count(FULL_2P_A, FULL_2P_B, 2057 * 6 + 3);
    let ref_cap = load_capture(FULL_2P_A).unwrap();
    let ref_b_cap = load_capture(FULL_2P_B).unwrap();
    assert_eq!(ref_cap.frames.len(), 21300);
    let ref_cmp = compare(
        &scen,
        &ref_cap,
        &ref_b_cap,
        CandidateKind::Reference,
        &CompareScope::default(),
    )
    .unwrap();
    assert!(
        matches!(
            ref_cmp,
            Outcome::Match {
                frames_compared: 21300,
                checkpoints_compared: 2057,
            }
        ),
        "{ref_cmp:?}"
    );

    // 2. No score ranks (checked in reference before running native):
    //    - No name-entry tune (0x38) or ranking-insertion tune (0x39) appears in frames.tsv.
    //    - In every captured checkpoint work-RAM dump, 0xff8e66..0xff8e6a (ranking row pointer)
    //      stays 0 and timer 2 (0xff8e16 & 4) is never armed.
    //    - The 160-byte ranking table 0xff8e6e..=0xff8f0d at session end (checkpoint 21299)
    //      equals the boot fill at checkpoint 8.
    for row in &ref_cap.frames {
        assert!(
            row.latch != "0x38" && row.latch != "0x39",
            "unexpected name-entry or ranking-insertion tune {} at frame {}",
            row.latch,
            row.frame
        );
    }
    let boot_work = std::fs::read(Path::new(FULL_2P_A).join("ram_000008_work.bin")).unwrap();
    let end_work = std::fs::read(Path::new(FULL_2P_A).join("ram_021299_work.bin")).unwrap();
    assert_eq!(
        &end_work[0x0e6e..0x0f0e],
        &boot_work[0x0e6e..0x0f0e],
        "ranking table 0xff8e6e..=0xff8f0d at session end must equal boot fill"
    );
    for &cp in &scen.checkpoints {
        let w = std::fs::read(Path::new(FULL_2P_A).join(format!("ram_{cp:06}_work.bin"))).unwrap();
        assert_eq!(
            &w[0x0e66..0x0e6a],
            &[0, 0, 0, 0],
            "reference checkpoint {cp}: 0xff8e66 must remain 0 (no qualifying score)"
        );
        assert_eq!(
            w[0x0e16] & 0x04,
            0,
            "reference checkpoint {cp}: timer 2 (name entry) must never be armed"
        );
    }

    // 3. C-NAT: PowerOnMachine from frame 0 through 21299 matches owned work RAM at every
    //    checkpoint and verifies all session milestones.
    let owned = cpu_owned_addresses();
    let checkpoint_set: BTreeSet<u32> = scen.checkpoints.iter().map(|&c| c as u32).collect();
    let mut slot2_at_2000 = None;
    let mut slot2_at_2014 = None;
    let mut mode_at_2111 = None;
    let mut vs1_carry_at_6990 = None;
    let mut watch_at_7259 = None;
    let mut challenger_at_7261 = None;
    let mut vs2_carry_at_12259 = None;
    let mut cpu1_lose_at_16533 = None;
    let mut continue_at_16556 = None;
    let mut attract_return_at_21213 = None;

    let (stopped, unrecovered) = observe_power_on_scenario(
        assets.maincpu.clone(),
        Path::new(FULL_2P_SCENARIO),
        21299,
        |m| {
            let f = m.state.frame;
            if checkpoint_set.contains(&f) && f >= 8 {
                let dump = std::fs::read(Path::new(FULL_2P_A).join(format!("ram_{f:06}_work.bin")))
                    .unwrap();
                let image = m.work_image();
                for &(name, addr) in &owned {
                    let i = (addr - 0xff8000) as usize;
                    assert_eq!(
                        dump[i], image[i],
                        "sf-full-2p-01 checkpoint {f}: {name} at 0x{addr:06x}"
                    );
                }
            }
            let w = m.work_ram();
            match f {
                2000 => slot2_at_2000 = Some((w[0x0003], w[0x0026], w[0x4008])),
                2014 => slot2_at_2014 = Some((w[0x0003], w[0x0026], w[0x065d])),
                2111 => mode_at_2111 = Some((w[0x4002], w[0x065d])),
                6990 => {
                    let who = u32::from_be_bytes([w[0x0e5e], w[0x0e5f], w[0x0e60], w[0x0e61]]);
                    let p2_score = w[0x06fc..0x0704].to_vec();
                    vs1_carry_at_6990 = Some((w[0x4002], who, p2_score));
                }
                7259 => {
                    let watch = u32::from_be_bytes([w[0x0f24], w[0x0f25], w[0x0f26], w[0x0f27]]);
                    watch_at_7259 = Some((w[0x4002], w[0x4008], w[0x065d], watch));
                }
                7261 => {
                    let watch = u32::from_be_bytes([w[0x0f24], w[0x0f25], w[0x0f26], w[0x0f27]]);
                    challenger_at_7261 = Some((w[0x4008], w[0x065d], watch));
                }
                12259 => {
                    let who = u32::from_be_bytes([w[0x0e5e], w[0x0e5f], w[0x0e60], w[0x0e61]]);
                    let p1_score = w[0x06d2..0x06da].to_vec();
                    let p2_score = w[0x06fc..0x0704].to_vec();
                    vs2_carry_at_12259 = Some((w[0x4002], who, p1_score, p2_score));
                }
                16533 => {
                    let who = u32::from_be_bytes([w[0x0e5e], w[0x0e5f], w[0x0e60], w[0x0e61]]);
                    let p1_score = w[0x06d2..0x06da].to_vec();
                    cpu1_lose_at_16533 = Some((w[0x4008], who, p1_score, w[0x065d]));
                }
                16556 => continue_at_16556 = Some((w[0x065d], w[0x0e36])),
                21213 => {
                    attract_return_at_21213 =
                        Some((w[0x0003], w[0x0026], w[0x0036], w[0x4002], w[0x4008]))
                }
                _ => {}
            }
        },
    )
    .unwrap();
    assert_eq!((stopped, unrecovered), (21299, vec![]));

    // Verify each milestone of the 2P session:
    assert_eq!(
        slot2_at_2000,
        Some((0x00, 0x08, 0x80)),
        "at 2000 (demo fight active), MODE_DEMO == 0, slot 2 is active (0x08), SCENE_MODE == 0x80"
    );
    assert_eq!(
        slot2_at_2014,
        Some((0xff, 0x00, 1)),
        "at 2014, first coin kills occupied slot 2 at 0x4823e (0xff8026 = 0) and enters start screen (MODE_DEMO = 0xff, 1 credit)"
    );
    assert_eq!(
        mode_at_2111,
        Some((1, 0)),
        "at 2111, start2 enters 2P versus (GAME_MODE = 1, 0 credits)"
    );
    assert_eq!(
        vs1_carry_at_6990,
        Some((4, 0xff86b0, vec![0, 0, 0, 4, 8, 8, 0, 0])),
        "at 6990, P2 Ken wins VS1 (score 4,880 < 5,000), 0x3eebc checks P1 Ryu (0xff86b0), and enters GAME_MODE = 4"
    );
    assert_eq!(
        watch_at_7259,
        Some((4, 0x80, 1, 0x3f47e)),
        "at 7259, slot 3 watch is armed at 0x3f47e in GAME_MODE = 4 (0x3f448 D2 = 1) with 1 credit"
    );
    assert_eq!(
        challenger_at_7261,
        Some((0x10, 0, 0x3f4c6)),
        "at 7261, start1 triggers P1 challenger join into P2's mode-4 fight (SCENE_MODE = 0x10, 0 credits, watch = 0x3f4c6)"
    );
    assert_eq!(
        vs2_carry_at_12259,
        Some((2, 0xff86da, vec![0, 0, 0, 4, 8, 8, 0, 0], vec![0; 8])),
        "at 12259, P1 Ryu wins VS2 (score 4,880 < 5,000), 0x3ee8a checks P2 Ken (0xff86da) and clears Ken's 4,880 score to 0, and enters GAME_MODE = 2"
    );
    assert_eq!(
        cpu1_lose_at_16533,
        Some((0x08, 0xff86b0, vec![0; 8], 1)),
        "at 16533, P1 loses CPU1 to Retsu, 0x3ef18 clears P1's 4,880 score to 0, and enters continue (SCENE_MODE = 8, 1 credit)"
    );
    assert_eq!(
        continue_at_16556,
        Some((0, 6)),
        "at 16556, start1 continues (0x3f146 -> 0x3f166: 0 credits)"
    );
    assert_eq!(
        attract_return_at_21213,
        Some((0, 0, 8, 0, 0)),
        "at 21213, game_over_reset kills slot 2 (0xff8026 = 0), clears MODE_DEMO/GAME_MODE/SCENE_MODE, and enters 0x3ccae (slot 3 active at 0xff8036 = 8)"
    );

    // Full capture comparison over 0..=21299 (21,300 frames, 2,057 checkpoints).
    let out = ScratchDir::new("sf-full-2p-01-native");
    let executed = run_power_on_scenario_capture_for_reference(
        &assets,
        Path::new(FULL_2P_A),
        &out,
        "sf-full-2p-01",
        Path::new(FULL_2P_SCENARIO),
        21299,
    )
    .unwrap();
    assert_eq!(executed, 21300);
    let nat_cmp = compare(
        &scen,
        &ref_cap,
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &CompareScope {
            from: Some(0),
            to: Some(21299),
            fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
        },
    )
    .unwrap();
    assert!(
        matches!(
            nat_cmp,
            Outcome::Match {
                frames_compared: 21300,
                checkpoints_compared: 2057,
            }
        ),
        "{nat_cmp:?}"
    );

    // Also verify frame 0's partial work-RAM clear prefix matches RESET_CLEAR_FRAME0_END.
    let f0_work = std::fs::read(Path::new(FULL_2P_A).join("ram_000000_work.bin")).unwrap();
    let clear_end = (RESET_CLEAR_FRAME0_END - 0xff8000) as usize;
    assert!(f0_work[..clear_end].iter().all(|&b| b == 0));

    // 4. C-NEG: moving one input by 1 frame (coin1 from 2000 to 2001) is caught at its
    //    first differing frame (2002, when vblank's debounce_coins at 0x3b2c4 sees coin1
    //    1 frame later at 0xff863f, and on screen/sound when slot 7 awards the credit 1
    //    frame later at 2015 vs 2014).
    let neg_dir = ScratchDir::new("sf-full-2p-01-neg-scen");
    std::fs::create_dir_all(&neg_dir).unwrap();
    let neg_scen_path = neg_dir.join("sf_full_2p_01_neg.json");
    let neg_json = std::fs::read_to_string(FULL_2P_SCENARIO).unwrap().replacen(
        "\"frame\": 2000,",
        "\"frame\": 2001,",
        1,
    );
    std::fs::write(&neg_scen_path, neg_json).unwrap();

    let mut first_ram_diff = None;
    observe_power_on_scenario(assets.maincpu.clone(), &neg_scen_path, 2020, |m| {
        let f = m.state.frame;
        if (2000..=2020).contains(&f) && first_ram_diff.is_none() {
            let ref_w =
                std::fs::read(Path::new(FULL_2P_A).join(format!("ram_{f:06}_work.bin"))).unwrap();
            let img = m.work_image();
            for &(name, addr) in &owned {
                let i = (addr - 0xff8000) as usize;
                if ref_w[i] != img[i] {
                    first_ram_diff = Some((f, name, addr, ref_w[i], img[i]));
                    break;
                }
            }
        }
    })
    .unwrap();
    assert_eq!(
        first_ram_diff,
        Some((2001, "coin slots, credits", 0xff863a, 0x01, 0x00)),
        "shifting coin1 from 2000 to 2001 is caught at frame 2001 on coin1 switch state 0xff863a"
    );

    let neg_out = ScratchDir::new("sf-full-2p-01-neg-cap");
    run_power_on_scenario_capture_for_reference(
        &assets,
        Path::new(FULL_2P_A),
        &neg_out,
        "sf-full-2p-01",
        &neg_scen_path,
        2020,
    )
    .unwrap();
    let neg_cap = load_capture(&neg_out).unwrap();
    let neg_fields = CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap();
    let ok_2012 = compare(
        &scen,
        &ref_cap,
        &neg_cap,
        CandidateKind::Native,
        &CompareScope {
            from: Some(0),
            to: Some(2012),
            fields: Some(neg_fields.clone()),
        },
    )
    .unwrap();
    assert!(
        matches!(
            ok_2012,
            Outcome::Match {
                frames_compared: 2013,
                ..
            }
        ),
        "{ok_2012:?}"
    );
    let bad_2013 = compare(
        &scen,
        &ref_cap,
        &neg_cap,
        CandidateKind::Native,
        &CompareScope {
            from: Some(0),
            to: Some(2013),
            fields: Some(neg_fields),
        },
    )
    .unwrap();
    assert!(
        matches!(
            bad_2013,
            Outcome::Mismatch(ref m)
                if m.frame == 2013
                    && (m.observable == "checkpoint videoram"
                        || m.observable == "checkpoint screen pixels")
        ),
        "expected mismatch at frame 2013, got {bad_2013:?}"
    );
}
