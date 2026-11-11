//! Ticket #137 (`F65`): a complete one-player game from power-on (`sf-full-1p-01`):
//!
//! Power-on (`0x3b000`) -> warning (`8..=793`) -> attract (`794..=1513`) ->
//! `coin1` at `1500` (`0xff865d = 1`, start screen at `1514`) -> `start1` at
//! `1620` (`GAME_MODE = 2`, `0xff865d = 0`, two-country select) -> Japan
//! (`0xffc006 = 0` at `1865`) -> Stage 0 Retsu (`0x1698c`, `2389..=5013`) ->
//! Stage 0 Geki (`0x16bfc`, `5014..=9018`) -> Japan power/timing bonus
//! (`0x40d30`, tier-1 break `+3,000`, `9019..=9597`) -> flight to USA ->
//! Stage 1 Joe (`0x1734c`, `10066..=14033`) -> Stage 1 Mike (`0x170dc`,
//! `14034..=17483`) -> Stage-1 10-second bonus (`0x41740`, timeout `+0`,
//! `17484..=18528`) -> flight to China -> Stage 2 Lee (`0x175bc`,
//! `19376..=23161`) -> Stage 2 Gen (`0x1782c`, `23162..=26070`) -> Stage-2
//! power/timing bonus (`0x41a02`, tier-2 break `+3,000`, `26071..=26487`) ->
//! flight to England -> Stage 3 Birdie (`0x17a9c`, `26956..=29775`) -> Stage 3 Eagle
//! (`0x17d0c`, `29776..=32314`) -> Stage-3 10-second bonus (`0x41fe6`, timeout
//! `+0`, `32315..=33359`) -> flight to Thailand -> Stage 4 Adon (`0x17f7c`,
//! `34184..=37152`) -> Stage 4 Sagat (`0x181ec`, `37153..=39775`) -> Ending
//! (`0x44a40..0x44c1c`, 5 country scenes `39777..=40977`, congratulatory
//! message `40978..=41943`, 8-page staff roll & `THE END` `41944..=43617`) ->
//! `0x3efc4` (`STAGE_NO: 4 -> 5`) -> `0x3f2d0` (`SCENE_MODE = 2`) -> `0x42422`
//! (`ranking_check`, rank 1 `0xff8e6e` with `605,700` pts) -> `0x42486`
//! (`name_entry_start`, `43617..=43693`, three `p1_lp` presses at `43630`,
//! `43655`, `43680` committing `AAA` via `0x427e2 -> 0x42920`) -> `0x425c4`
//! (`name_entry_insert` at `43694`) -> `0x4268c` (`ranking_screen_start`,
//! 3-second marked-entry blink `0x42750..0x427e0`, `43695..=43900`) ->
//! `0x3f2e0` (`versus_game_over_p2`, `SCENE_MODE = 1`) -> `0x3f2f0`
//! (`game_over_screen`, `43901..=44141`) -> `0x3f33c` (`game_over_reset` at
//! `44142`, killing slot 2 and spawning slot 3 at `0x3f374`) -> `0x3ccc2 /
//! 0x3ccec` (return to attract `44142..=44199`).

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

const FULL_1P_SCENARIO: &str = "../../tools/reference/scenarios/sf_full_1p_01.json";
const WIN_10_SCENARIO: &str = "../../tools/reference/scenarios/sf_win_10.json";
const FULL_1P_A: &str = "../../tools/reference/local/captures/full-1p-01-run-a";
const FULL_1P_B: &str = "../../tools/reference/local/captures/full-1p-01-run-b";
const WIN_10_A: &str = "../../tools/reference/local/captures/win-10-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn present() -> bool {
    [FULL_1P_A, FULL_1P_B, WIN_10_A, ROMS]
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
fn full_1p_game_from_power_on_matches_reference_and_catches_1_frame_input_shift() {
    if !present() {
        eprintln!("skipping: local sf-full-1p-01 / sf-win-10 captures or ROMs not present");
        return;
    }

    let assets = load_assets(ROMS, MANIFEST).unwrap();
    let scen = load_scenario(FULL_1P_SCENARIO).unwrap();
    let win10_scen = load_scenario(WIN_10_SCENARIO).unwrap();
    assert_eq!(scen.duration_frames, 44200);
    assert_eq!(scen.checkpoints.len(), 873);
    assert_eq!((scen.dsw1, scen.dsw2), (0xdfff, 0xffff));

    // 1. C-REF:
    //    - full-1p-01-run-a == full-1p-01-run-b across all 5,241 observation files
    //      (873 checkpoints * 6 files + 3 TSVs).
    //    - Prefix-equal to sf-win-10 through frame 43618: inputs[..2077] equal
    //      sf-win-10's inputs, frames.tsv rows 0..=43618 equal win-10-run-a's,
    //      and all 55 shared checkpoints in 37153..=43618 match win-10-run-a
    //      byte-for-byte.
    assert_runs_identical_with_count(FULL_1P_A, FULL_1P_B, 873 * 6 + 3);
    assert_eq!(scen.inputs.len(), win10_scen.inputs.len() + 3);
    for (a, b) in scen.inputs[..win10_scen.inputs.len()]
        .iter()
        .zip(&win10_scen.inputs)
    {
        assert_eq!(
            (a.frame, &a.press, a.hold_frames),
            (b.frame, &b.press, b.hold_frames)
        );
    }

    let ref_cap = load_capture(FULL_1P_A).unwrap();
    let ref_b_cap = load_capture(FULL_1P_B).unwrap();
    let win10_cap = load_capture(WIN_10_A).unwrap();
    assert_eq!(ref_cap.frames.len(), 44200);
    assert_eq!(win10_cap.frames.len(), 43619);
    for (r, w) in ref_cap.frames[..43619].iter().zip(&win10_cap.frames) {
        assert_eq!(
            (&r.frame, &r.screen_hash, &r.latch, &r.latch_changed),
            (&w.frame, &w.screen_hash, &w.latch, &w.latch_changed)
        );
    }

    let win10_cp_set: BTreeSet<u64> = win10_scen.checkpoints.iter().copied().collect();
    let mut shared_cps = 0usize;
    for &cp in &scen.checkpoints {
        if win10_cp_set.contains(&cp) {
            shared_cps += 1;
            for suffix in [
                format!("pixels_{cp:06}.png"),
                format!("pixels_{cp:06}.rgba"),
                format!("ram_{cp:06}_work.bin"),
                format!("ram_{cp:06}_video.bin"),
                format!("ram_{cp:06}_palette.bin"),
                format!("ram_{cp:06}_music.bin"),
            ] {
                assert_eq!(
                    std::fs::read(Path::new(FULL_1P_A).join(&suffix)).unwrap(),
                    std::fs::read(Path::new(WIN_10_A).join(&suffix)).unwrap(),
                    "shared checkpoint {cp} file {suffix} must equal win-10-run-a"
                );
            }
        }
    }
    assert_eq!(shared_cps, 55);

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
                frames_compared: 44200,
                checkpoints_compared: 873,
            }
        ),
        "{ref_cmp:?}"
    );

    // 2 & 3. C-NAT from power-on (no fixture, including random state 0xff8e4a..=0xff8e51
    //        from frame 0) through the return to the attract (44199), with zero
    //        untranslated branches reached.
    let owned = cpu_owned_addresses();
    assert!(
        (0xff8e4a..=0xff8e51).all(|addr| owned.iter().any(|&(_, a)| a == addr)),
        "cpu_owned_addresses must include all 8 PRNG bytes 0xff8e4a..=0xff8e51"
    );
    let checkpoint_set: BTreeSet<u32> = scen.checkpoints.iter().map(|&c| c as u32).collect();

    let mut boot_at_8 = None;
    let mut attract_at_794 = None;
    let mut start_at_1514 = None;
    let mut retsu_at_2389 = None;
    let mut geki_at_5860 = None;
    let mut bonus0_at_9278 = None;
    let mut joe_at_10840 = None;
    let mut mike_at_13712 = None;
    let mut bonus1_at_17484 = None;
    let mut lee_at_19379 = None;
    let mut gen_at_22990 = None;
    let mut bonus2_at_25464 = None;
    let mut birdie_at_26956 = None;
    let mut eagle_at_29601 = None;
    let mut bonus3_at_32316 = None;
    let mut adon_at_34183 = None;
    let mut sagat_at_37153 = None;
    let mut ending_at_39776 = None;
    let mut name_entry_at_43618 = None;
    let mut rank_insert_at_43694 = None;
    let mut game_over_at_43902 = None;
    let mut attract_return_at_44143 = None;

    let (stopped, unrecovered) = observe_power_on_scenario(
        assets.maincpu.clone(),
        Path::new(FULL_1P_SCENARIO),
        44199,
        |m| {
            let f = m.state.frame;
            if checkpoint_set.contains(&f) && f >= 8 {
                let dump = std::fs::read(Path::new(FULL_1P_A).join(format!("ram_{f:06}_work.bin")))
                    .unwrap();
                let image = m.work_image();
                for &(name, addr) in &owned {
                    let i = (addr - 0xff8000) as usize;
                    assert_eq!(
                        dump[i], image[i],
                        "sf-full-1p-01 checkpoint {f}: {name} at 0x{addr:06x}"
                    );
                }
            }
            let w = m.work_ram();
            let opp_mv = u32::from_be_bytes([w[0x4676], w[0x4677], w[0x4678], w[0x4679]]);
            match f {
                8 => boot_at_8 = Some((w[0x0e6e..0x0e7e].to_vec(), w[0x4012..0x401a].to_vec())),
                794 => attract_at_794 = Some((w[0x0003], w[0x0016], w[0x0036])),
                1514 => start_at_1514 = Some((w[0x0003], w[0x065d])),
                2389 => retsu_at_2389 = Some((w[0x4002], w[0x4003], w[0x4004], opp_mv)),
                5860 => geki_at_5860 = Some((w[0x4003], w[0x4004], opp_mv)),
                9278 => bonus0_at_9278 = Some((w[0x4003], w[0x4008])),
                10840 => joe_at_10840 = Some((w[0x4003], w[0x4004], opp_mv)),
                13712 => mike_at_13712 = Some((w[0x4003], w[0x4004], opp_mv)),
                17484 => bonus1_at_17484 = Some((w[0x4003], w[0x4008])),
                19379 => lee_at_19379 = Some((w[0x4003], w[0x4004], opp_mv)),
                22990 => gen_at_22990 = Some((w[0x4003], w[0x4004], opp_mv)),
                25464 => bonus2_at_25464 = Some((w[0x4003], w[0x4008])),
                26956 => birdie_at_26956 = Some((w[0x4003], w[0x4004], opp_mv)),
                29601 => eagle_at_29601 = Some((w[0x4003], w[0x4004], opp_mv)),
                32316 => bonus3_at_32316 = Some((w[0x4003], w[0x4008])),
                34183 => adon_at_34183 = Some((w[0x4003], w[0x4004], opp_mv)),
                37153 => sagat_at_37153 = Some((w[0x4003], w[0x4004], opp_mv)),
                39776 => ending_at_39776 = Some((w[0x4003], w[0x4004], w[0x0682])),
                43618 => {
                    let row_ptr = u32::from_be_bytes([w[0x0e66], w[0x0e67], w[0x0e68], w[0x0e69]]);
                    let p1_score = w[0x06d2..0x06da].to_vec();
                    name_entry_at_43618 = Some((w[0x4003], w[0x4008], row_ptr, p1_score));
                }
                43694 => {
                    let rank1 = w[0x0e6e..0x0e7e].to_vec();
                    let p1_score = w[0x06d2..0x06da].to_vec();
                    rank_insert_at_43694 = Some((rank1, p1_score));
                }
                43902 => {
                    let rank1_mark = w[0x0e72];
                    game_over_at_43902 = Some((w[0x4003], w[0x4008], rank1_mark));
                }
                44143 => {
                    attract_return_at_44143 = Some((
                        w[0x0003], w[0x0026], w[0x0036], w[0x4002], w[0x4003], w[0x4008],
                    ));
                }
                _ => {}
            }
        },
    )
    .unwrap();
    assert_eq!((stopped, unrecovered), (44199, vec![]));

    // Verify all 1P full-game milestones:
    assert_eq!(
        boot_at_8,
        Some((
            vec![0x04, 0xa0, 0x04, 0xb4, 0, 0, 0, 1, 0, 1, 8, 0, 0, 0x11, 0x1b, 0x18],
            vec![0, 0, 1, 0, 1, 8, 0, 0],
        )),
        "frame 8: boot ranking fill (1ST = 101,800 CAP) and top score = 101,800"
    );
    assert_eq!(attract_at_794, Some((0x00, 0x08, 0x08)));
    assert_eq!(start_at_1514, Some((0xff, 1)));
    assert_eq!(retsu_at_2389, Some((2, 0, 0, 0x16e6c)));
    assert_eq!(geki_at_5860, Some((0, 1, 0x16bfc)));
    assert_eq!(bonus0_at_9278, Some((0, 0x40)));
    assert_eq!(joe_at_10840, Some((1, 0, 0x1734c)));
    assert_eq!(mike_at_13712, Some((1, 1, 0x170dc)));
    assert_eq!(bonus1_at_17484, Some((1, 0x40)));
    assert_eq!(lee_at_19379, Some((2, 0, 0x175bc)));
    assert_eq!(gen_at_22990, Some((2, 1, 0x1782c)));
    assert_eq!(bonus2_at_25464, Some((2, 0xc0)));
    assert_eq!(birdie_at_26956, Some((3, 0, 0x17a9c)));
    assert_eq!(eagle_at_29601, Some((3, 1, 0x17d0c)));
    assert_eq!(bonus3_at_32316, Some((3, 0x40)));
    assert_eq!(adon_at_34183, Some((4, 0, 0x17f7c)));
    assert_eq!(sagat_at_37153, Some((4, 1, 0x181ec)));
    assert_eq!(ending_at_39776, Some((4, 0, 0x00)));
    assert_eq!(
        name_entry_at_43618,
        Some((5, 2, 0xff8e6e, vec![0, 0, 4, 9, 4, 3, 0, 0])),
        "frame 43618: stage 5 name entry armed for P1 rank 1 (0xff8e6e) with 494,300 pts"
    );
    assert_eq!(
        rank_insert_at_43694,
        Some((
            vec![0x04, 0xa0, 0x04, 0xb4, 0xff, 0, 0, 4, 9, 4, 3, 0, 0, 0x0a, 0x0a, 0x0a],
            vec![0; 8],
        )),
        "frame 43694: 0x425c4 inserts rank 1 (marked 0xff, 494,300 pts, initials AAA = 0x0a,0x0a,0x0a) and clears P1 score"
    );
    assert_eq!(
        game_over_at_43902,
        Some((5, 1, 0)),
        "frame 43902: marked ranking blink clears 0xff8e72 = 0 and enters 0x3f2e0 -> 0x3f2f0 game-over screen (SCENE_MODE = 1)"
    );
    assert_eq!(
        attract_return_at_44143,
        Some((0, 0, 8, 0, 0, 0)),
        "frame 44143: 0x3f33c game_over_reset kills slot 2, clears game/stage/scene state, and returns to attract (0x3ccec)"
    );

    // Also verify frame 0's partial work-RAM clear prefix matches RESET_CLEAR_FRAME0_END.
    let f0_work = std::fs::read(Path::new(FULL_1P_A).join("ram_000000_work.bin")).unwrap();
    let clear_end = (RESET_CLEAR_FRAME0_END - 0xff8000) as usize;
    assert!(f0_work[..clear_end].iter().all(|&b| b == 0));

    // 4. C-NEG: moving one input by 1 frame (start1 shifted from 1520 to 1519) is caught
    //    at its first differing frame (1520 on owned work RAM and checkpoint
    //    videoram/pixels when 1P start triggers 1 frame early).
    let neg_dir = ScratchDir::new("sf-full-1p-01-neg-scen");
    std::fs::create_dir_all(&neg_dir).unwrap();
    let neg_start_path = neg_dir.join("sf_full_1p_01_neg_start.json");
    let neg_start_json = std::fs::read_to_string(FULL_1P_SCENARIO).unwrap().replacen(
        "\"frame\": 1520,",
        "\"frame\": 1519,",
        1,
    );
    std::fs::write(&neg_start_path, neg_start_json).unwrap();

    let mut first_ram_diff = None;
    observe_power_on_scenario(assets.maincpu.clone(), &neg_start_path, 1520, |m| {
        let f = m.state.frame;
        if (1498..=1520).contains(&f) && first_ram_diff.is_none() {
            let ref_w =
                std::fs::read(Path::new(FULL_1P_A).join(format!("ram_{f:06}_work.bin"))).unwrap();
            let img = m.work_image();
            for &(name, addr) in &owned {
                let i = (addr - 0xff8000) as usize;
                if ref_w[i] != img[i] {
                    first_ram_diff = Some((f, name, addr));
                    break;
                }
            }
        }
    })
    .unwrap();
    assert_eq!(
        first_ram_diff.map(|(f, _, _)| f),
        Some(1520),
        "shifting start1 from 1520 to 1519 is caught in owned work RAM at frame 1520: {first_ram_diff:?}"
    );

    let neg_out = ScratchDir::new("sf-full-1p-01-neg-cap");
    run_power_on_scenario_capture_for_reference(
        &assets,
        Path::new(FULL_1P_A),
        &neg_out,
        "sf-full-1p-01",
        &neg_start_path,
        1520,
    )
    .unwrap();
    let neg_cap = load_capture(&neg_out).unwrap();
    let neg_fields = CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap();
    let ok_1519 = compare(
        &scen,
        &ref_cap,
        &neg_cap,
        CandidateKind::Native,
        &CompareScope {
            from: Some(0),
            to: Some(1519),
            fields: Some(neg_fields.clone()),
        },
    )
    .unwrap();
    assert!(
        matches!(
            ok_1519,
            Outcome::Match {
                frames_compared: 1520,
                checkpoints_compared: 49
            }
        ),
        "{ok_1519:?}"
    );
    let bad_1520 = compare(
        &scen,
        &ref_cap,
        &neg_cap,
        CandidateKind::Native,
        &CompareScope {
            from: Some(0),
            to: Some(1520),
            fields: Some(neg_fields.clone()),
        },
    )
    .unwrap();
    assert!(
        matches!(
            bad_1520,
            Outcome::Mismatch(ref m)
                if m.frame == 1520
                    && (m.observable == "checkpoint videoram"
                        || m.observable == "checkpoint screen pixels")
        ),
        "expected mismatch at frame 1520, got {bad_1520:?}"
    );

    // Full capture comparison over 0..=44199 (44,200 frames, 873 checkpoints).
    let out = ScratchDir::new("sf-full-1p-01-native");
    let executed = run_power_on_scenario_capture_for_reference(
        &assets,
        Path::new(FULL_1P_A),
        &out,
        "sf-full-1p-01",
        Path::new(FULL_1P_SCENARIO),
        44199,
    )
    .unwrap();
    assert_eq!(executed, 44200);
    let nat_cmp = compare(
        &scen,
        &ref_cap,
        &load_capture(&out).unwrap(),
        CandidateKind::Native,
        &CompareScope {
            from: Some(0),
            to: Some(44199),
            fields: Some(neg_fields),
        },
    )
    .unwrap();
    assert!(
        matches!(
            nat_cmp,
            Outcome::Match {
                frames_compared: 44200,
                checkpoints_compared: 873,
            }
        ),
        "{nat_cmp:?}"
    );
}
