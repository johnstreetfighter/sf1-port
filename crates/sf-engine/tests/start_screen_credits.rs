//! Start-screen credits, service switch, coin counter pulses, 9-credit
//! lockout, and 1P/2P start verification (ticket #52, F12).
//!
//! Verifies:
//! 1. `C-REF`: `sf-credit-select-01`, `sf-credits-01` and `sf-start-2p-01`
//!    each produce two byte-identical MAME 0.289 captures (`run-a == run-b`)
//!    and match `fight-run-a` / `sf-move-01` on their shared prefixes.
//! 2. `C-NAT`: from each capture's `1500` checkpoint (`0x3dfce`) through its
//!    start press (`1501..=1520` on `sf-credit-select-01`, `1501..=2140` on
//!    `sf-credits-01`, `1501..=1560` on `sf-start-2p-01`), the native engine
//!    matches the reference capture on `screen_hash`, `latch`,
//!    `latch_changed`, checkpoint pixels, checkpoint RAM (`video`, `object`,
//!    `palette`), owned work RAM (`compare_cpu_ram_with_inputs`, including
//!    `0xff863a..0xff865e`, `0xff8684..0xff8686` and `0xffc01a..0xffc01c`),
//!    and full work RAM (`ram_crc` and non-stack bytes of `0xff8000..0xffffff`).
//! 3. Coin-counter pulse's 6-frame sleep (`0x48344`) and service path's
//!    6-frame debounce (`0x481e8`) + 30-frame cooldown (`0x48212`) match
//!    slot 7's bytes and random-state timing (`0xff8e4a..0xff8e52`).
//! 4. At 9 credits (`sf-credits-01` frame `2010`), lockout clears `0x7070`
//!    (`0xff8684 = 0x0000`, `state.coin_ctrl = 0x0000`) and a coin pressed
//!    during lockout (`coin1` at `2080`) is suppressed as in the reference.
//! 5. Starts: 1P start (`sf-credit-select-01` at `1521`, `sf-credits-01` at
//!    `2141`) gives mode `2` and takes 1 credit; 2P start with 1 credit
//!    (`sf-credits-01` at `1541..=1552`) is ignored; 2P start with 2 credits
//!    (`sf-start-2p-01` at `1561`) gives mode `1` and takes 2 credits; the
//!    exit erases `0x3e038`'s five blocks even on a 1-credit start; `0x3e04c`
//!    is reported (`C-STOP`).
//! 6. `C-NEG`: delaying a coin press by 10 frames (`coin2` at `1590` instead
//!    of `1580` in `sf-credits-01`) is caught by both
//!    `compare_cpu_ram_with_inputs` and `sf_compare::compare`.
//! 7. `sf_fixed_fight_01.json`'s `checkpoint_notes` describes `1500` as the
//!    start screen with slot 1 at `0x3dfce`.

use std::path::{Path, PathBuf};

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::cpu::CpuMachine;
use sf_engine::run::{
    compare_cpu_ram_with_inputs, cpu_work_image, observe_cpu_fight,
    run_cpu_fight_capture_with_inputs, CpuRamOutcome, ScratchDir,
};
use sf_engine::tasks::Scheduler;
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const CREDIT_SELECT_SCENARIO: &str = "../../tools/reference/scenarios/sf_credit_select_01.json";
const CREDITS_SCENARIO: &str = "../../tools/reference/scenarios/sf_credits_01.json";
const START_2P_SCENARIO: &str = "../../tools/reference/scenarios/sf_start_2p_01.json";
const FIXED_FIGHT_SCENARIO: &str = "../../tools/reference/scenarios/sf_fixed_fight_01.json";

const CREDIT_SELECT_A: &str = "../../tools/reference/local/captures/credit-select-01-run-a";
const CREDIT_SELECT_B: &str = "../../tools/reference/local/captures/credit-select-01-run-b";
const CREDITS_A: &str = "../../tools/reference/local/captures/credits-01-run-a";
const CREDITS_B: &str = "../../tools/reference/local/captures/credits-01-run-b";
const START_2P_A: &str = "../../tools/reference/local/captures/start-2p-01-run-a";
const START_2P_B: &str = "../../tools/reference/local/captures/start-2p-01-run-b";
const FIGHT_A: &str = "../../tools/reference/local/captures/fight-run-a";
const MOVE_A: &str = "../../tools/reference/local/captures/move-run-a";

fn local(rel: &str) -> Option<PathBuf> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    p.exists().then_some(p)
}

struct Captures {
    assets: sf_video::romset::Assets,
    cs_a: PathBuf,
    cs_b: PathBuf,
    cr_a: PathBuf,
    cr_b: PathBuf,
    s2_a: PathBuf,
    s2_b: PathBuf,
    fight_a: PathBuf,
    move_a: PathBuf,
}

fn require_captures() -> Option<Captures> {
    let (roms, cs_a, cs_b, cr_a, cr_b, s2_a, s2_b, fight_a, move_a) = (
        local(ROMS)?,
        local(CREDIT_SELECT_A)?,
        local(CREDIT_SELECT_B)?,
        local(CREDITS_A)?,
        local(CREDITS_B)?,
        local(START_2P_A)?,
        local(START_2P_B)?,
        local(FIGHT_A)?,
        local(MOVE_A)?,
    );
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join(MANIFEST);
    let assets = load_assets(&roms, &manifest).expect("ROM set assembles");
    Some(Captures {
        assets,
        cs_a,
        cs_b,
        cr_a,
        cr_b,
        s2_a,
        s2_b,
        fight_a,
        move_a,
    })
}

fn assert_dir_identical(a: &Path, b: &Path, label: &str) {
    let mut files_a: Vec<_> = std::fs::read_dir(a)
        .unwrap()
        .filter_map(|e| {
            let e = e.unwrap();
            e.file_type().unwrap().is_file().then(|| e.file_name())
        })
        .collect();
    let mut files_b: Vec<_> = std::fs::read_dir(b)
        .unwrap()
        .filter_map(|e| {
            let e = e.unwrap();
            e.file_type().unwrap().is_file().then(|| e.file_name())
        })
        .collect();
    files_a.sort();
    files_b.sort();
    assert_eq!(files_a, files_b, "{label}: file list differs");
    for name in files_a {
        if matches!(
            name.to_str(),
            Some("metadata.json" | "summary.json" | "generated_driver.lua")
        ) {
            continue;
        }
        let ba = std::fs::read(a.join(&name)).unwrap();
        let bb = std::fs::read(b.join(&name)).unwrap();
        assert_eq!(
            ba, bb,
            "{label}: {:?} differs between run-a and run-b",
            name
        );
    }
}

fn assert_checkpoint_identical(a: &Path, b: &Path, frame: u32, label: &str) {
    for name in [
        format!("pixels_{frame:06}.rgba"),
        format!("ram_{frame:06}_video.bin"),
        format!("ram_{frame:06}_palette.bin"),
        format!("ram_{frame:06}_work.bin"),
        format!("ram_{frame:06}_music.bin"),
    ] {
        let ba = std::fs::read(a.join(&name)).unwrap();
        let bb = std::fs::read(b.join(&name)).unwrap();
        assert_eq!(ba, bb, "{label}: {name} differs");
    }
}

fn assert_frames_prefix_identical(a: &Path, b: &Path, last_frame: u32, label: &str) {
    let cap_a = load_capture(a).unwrap();
    let cap_b = load_capture(b).unwrap();
    for f in 0..=last_frame as usize {
        let fa = &cap_a.frames[f];
        let fb = &cap_b.frames[f];
        assert_eq!(fa.frame, fb.frame, "{label}: frame index {f}");
        assert_eq!(
            fa.screen_hash, fb.screen_hash,
            "{label}: screen_hash row {f}"
        );
        assert_eq!(fa.latch, fb.latch, "{label}: latch row {f}");
        assert_eq!(
            fa.latch_changed, fb.latch_changed,
            "{label}: latch_changed row {f}"
        );
        assert_eq!(fa.ram_crc, fb.ram_crc, "{label}: ram_crc row {f}");
        assert_eq!(fa.music_crc, fb.music_crc, "{label}: music_crc row {f}");
    }
}

/// Criterion 7: `sf_fixed_fight_01.json`'s `checkpoint_notes` describes
/// `1500` as the start screen with slot 1 at `0x3dfce`.
#[test]
fn fixed_fight_scenario_notes_describe_1500_as_start_screen_at_0x3dfce() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXED_FIGHT_SCENARIO);
    let text = std::fs::read_to_string(&path).expect("read sf_fixed_fight_01.json");
    let json: serde_json::Value = serde_json::from_str(&text).expect("parse JSON");
    let notes = json["observations"]["checkpoint_notes"]
        .as_str()
        .expect("checkpoint_notes string");
    assert!(
        notes.contains("1500=start screen (slot 1 at 0x3dfce)"),
        "expected checkpoint_notes to describe 1500 as start screen (slot 1 at 0x3dfce), got: {notes}"
    );
}

/// Criterion 1 (`C-REF`): `sf_credit_select_01.json`, `sf_credits_01.json`
/// and `sf_start_2p_01.json` each produce two identical MAME 0.289 captures
/// and match `fight-run-a` / `sf-move-01` on their shared prefixes.
#[test]
fn reference_captures_are_reproducible_and_match_shared_prefixes() {
    let Some(Captures {
        cs_a,
        cs_b,
        cr_a,
        cr_b,
        s2_a,
        s2_b,
        fight_a,
        move_a,
        ..
    }) = require_captures()
    else {
        eprintln!("skipping: start-screen reference captures not present");
        return;
    };

    assert_dir_identical(&cs_a, &cs_b, "sf-credit-select-01");
    assert_dir_identical(&cr_a, &cr_b, "sf-credits-01");
    assert_dir_identical(&s2_a, &s2_b, "sf-start-2p-01");

    assert_frames_prefix_identical(&cs_a, &fight_a, 1870, "sf-credit-select-01 vs fight-run-a");
    assert_checkpoint_identical(
        &cs_a,
        &fight_a,
        1500,
        "sf-credit-select-01 vs fight-run-a @1500",
    );
    assert_checkpoint_identical(
        &cs_a,
        &fight_a,
        1860,
        "sf-credit-select-01 vs fight-run-a @1860",
    );

    assert_frames_prefix_identical(&cr_a, &fight_a, 1519, "sf-credits-01 vs fight-run-a");
    assert_checkpoint_identical(&cr_a, &fight_a, 1500, "sf-credits-01 vs fight-run-a @1500");

    assert_frames_prefix_identical(&s2_a, &fight_a, 1479, "sf-start-2p-01 vs fight-run-a");
    assert_frames_prefix_identical(&s2_a, &move_a, 2459, "sf-start-2p-01 vs sf-move-01");
    assert_checkpoint_identical(&s2_a, &move_a, 2450, "sf-start-2p-01 vs sf-move-01 @2450");
}

fn verify_native_interval(
    assets: &sf_video::romset::Assets,
    reference: &Path,
    scenario_path: &Path,
    scenario_id: &str,
    first: u32,
    last: u32,
) {
    let outcome = compare_cpu_ram_with_inputs(
        reference,
        assets.maincpu.clone(),
        Some(scenario_path),
        first,
        last,
    )
    .expect("read capture dumps");
    assert_eq!(
        outcome,
        CpuRamOutcome::Match {
            frames: last - first
        },
        "{scenario_id} owned work RAM {first}..={last}"
    );

    // Also verify every non-stack byte of the full 0xff8000..0xffffff work
    // dump across every frame of the interval, plus every scheduler task
    // decode from the reference dump.
    let (stopped, unrecovered) = observe_cpu_fight(
        reference,
        assets.maincpu.clone(),
        Some(scenario_path),
        first,
        last,
        |m| {
            let f = m.state.frame;
            let ref_work = std::fs::read(reference.join(format!("ram_{f:06}_work.bin")))
                .expect("reference work dump");
            let _ = Scheduler::from_fixture(&ref_work)
                .unwrap_or_else(|e| panic!("{scenario_id} frame {f} scheduler decode: {e}"));
            let native_work = cpu_work_image(m);
            for (i, (&r, &n)) in ref_work.iter().zip(native_work.iter()).enumerate() {
                let addr = 0xff8000 + i as u32;
                let in_task_tcb_saved = (0xff8008..0xff8106).contains(&addr)
                    && ((addr - 0xff8006) & 0x0f) >= 2;
                let in_task_stack = (0xff8120..0xff8622).contains(&addr);
                if !in_task_tcb_saved && !in_task_stack {
                    assert_eq!(
                        n, r,
                        "{scenario_id} frame {f}: full work RAM differs at 0x{addr:06x}: ref {r:02x}, native {n:02x}"
                    );
                }
            }
        },
    )
    .expect("observe_cpu_fight");
    assert_eq!((stopped, unrecovered), (last, Vec::new()));

    // Verify full capture observables (screen_hash, latch, latch_changed,
    // checkpoint pixels, video/object/palette RAM) via sf_compare::compare.
    let out = ScratchDir::new(&format!("sf-start-screen-{scenario_id}"));
    let executed = run_cpu_fight_capture_with_inputs(
        reference,
        assets,
        &out,
        scenario_id,
        Some(scenario_path),
        first,
        last,
    )
    .expect("write native capture");
    assert_eq!(executed, u64::from(last - first + 1));

    let scenario = load_scenario(scenario_path).expect("load scenario");
    let ref_cap = load_capture(reference).expect("load reference capture");
    let cand_cap = load_capture(&out).expect("load native capture");
    let scope = CompareScope::default()
        .with_range(u64::from(first + 1), u64::from(last))
        .with_fields(
            CompareScope::parse_fields("screen_hash,latch,latch_changed")
                .expect("valid observable field names"),
        );
    let outcome = compare(
        &scenario,
        &ref_cap,
        &cand_cap,
        CandidateKind::Native,
        &scope,
    )
    .expect("compare captures");
    match outcome {
        Outcome::Match {
            frames_compared,
            checkpoints_compared,
        } => {
            assert_eq!(frames_compared, u64::from(last - first));
            assert_eq!(checkpoints_compared, (last - first) as usize);
        }
        Outcome::Mismatch(m) => panic!("{scenario_id} compare mismatch: {m:?}"),
    }
}

/// Criterion 2 (`C-NAT`): from each capture's `1500` checkpoint (`0x3dfce`)
/// through its start press, the native engine matches the reference capture
/// on `screen_hash`, `latch`, `latch_changed`, checkpoint pixels, checkpoint
/// RAM and owned + full work RAM.
#[test]
fn start_screen_matches_reference_from_1500_through_each_start_press() {
    let Some(Captures {
        assets,
        cs_a,
        cr_a,
        s2_a,
        ..
    }) = require_captures()
    else {
        eprintln!("skipping: start-screen reference captures not present");
        return;
    };
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    verify_native_interval(
        &assets,
        &cs_a,
        &manifest_dir.join(CREDIT_SELECT_SCENARIO),
        "sf-credit-select-01",
        1500,
        1520,
    );
    verify_native_interval(
        &assets,
        &cr_a,
        &manifest_dir.join(CREDITS_SCENARIO),
        "sf-credits-01",
        1500,
        2140,
    );
    verify_native_interval(
        &assets,
        &s2_a,
        &manifest_dir.join(START_2P_SCENARIO),
        "sf-start-2p-01",
        1500,
        1560,
    );
}

/// Criteria 3 & 4:
/// - Coin-counter pulse's 6-frame sleep (`0x48344`) and service path's
///   6-frame debounce (`0x481e8`) + 30-frame cooldown (`0x48212`) match the
///   reference's slot-7 bytes and random-state timing.
/// - At 9 credits (`sf-credits-01` frame `2010`), lockout clears `0x7070`
///   (`0xff8684 = 0x0000`, `state.coin_ctrl = 0x0000`) and a coin pressed
///   during lockout (`coin1` at `2080`) is suppressed as in the reference.
#[test]
fn coin_pulses_service_debounce_cooldown_and_nine_credit_lockout_match_reference() {
    let Some(Captures { assets, cr_a, .. }) = require_captures() else {
        eprintln!("skipping: start-screen reference captures not present");
        return;
    };
    let scenario_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(CREDITS_SCENARIO);

    let mut pulse_frames = 0u32;
    let mut service_debounce_frames = 0u32;
    let mut service_cooldown_frames = 0u32;
    let mut lockout_frame = None;
    let mut credits_at_2095 = None;
    let mut slot1_debounce_at_2085 = None;

    let (stopped, unrecovered) = observe_cpu_fight(
        &cr_a,
        assets.maincpu.clone(),
        Some(&scenario_path),
        1500,
        2140,
        |m| {
            let f = m.state.frame;
            let w = m.work_ram();
            let rw = |addr: u32| -> u16 {
                let i = (addr - 0xff8000) as usize;
                u16::from_be_bytes([w[i], w[i + 1]])
            };
            let rb = |addr: u32| -> u8 { w[(addr - 0xff8000) as usize] };

            let ref_work = std::fs::read(cr_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
            let ref_sched = Scheduler::from_fixture(&ref_work).unwrap();
            let pos = ref_sched.positions();
            let slot7_resume = pos
                .iter()
                .find_map(|&(slot, resume)| (slot == 7).then_some(resume))
                .expect("slot 7 active");
            match slot7_resume {
                0x48344 => pulse_frames += 1,
                0x481e8 => service_debounce_frames += 1,
                0x48212 => service_cooldown_frames += 1,
                _ => {}
            }

            // Slot 7's state/sleep bytes and random state match the reference
            // on every frame.
            assert_eq!(rb(0xff8076), ref_work[0x0076], "frame {f}: slot 7 state");
            assert_eq!(rb(0xff8077), ref_work[0x0077], "frame {f}: slot 7 sleep");
            assert_eq!(
                &w[0x0e4a..0x0e52],
                &ref_work[0x0e4a..0x0e52],
                "frame {f}: random state 0xff8e4a..0xff8e52"
            );

            // Check coin_ctrl copy (0xff8684) and hardware register (state.coin_ctrl).
            let coin_ctrl_copy = rw(0xff8684);
            assert_eq!(
                m.state.coin_ctrl, coin_ctrl_copy,
                "frame {f}: hardware coin_ctrl tracks 0xff8684"
            );
            if coin_ctrl_copy == 0x0000 && lockout_frame.is_none() {
                lockout_frame = Some((f, rw(0xff865c)));
            }
            if f == 2085 {
                slot1_debounce_at_2085 = Some(rw(0xff863a));
            }
            if f == 2095 {
                credits_at_2095 = Some(rw(0xff865c));
            }
        },
    )
    .expect("observe_cpu_fight");
    assert_eq!((stopped, unrecovered), (2140, Vec::new()));

    // Two coin insertions before lockout (coin2 at 1580, coin1 at 1620), each
    // sleeping 6 frames inside the counter pulse (0x48344): 12 frames total.
    assert_eq!(
        pulse_frames, 12,
        "two 6-frame coin counter pulses at 0x48344"
    );
    // Six service1 credits (1660, 1720, 1780, 1840, 1900, 1962) before the
    // 9-credit cap: each sleeps 6 frames in debounce (0x481e8) and 30 frames
    // in cooldown (0x48212). Once at 9 credits (2010), 0x481ca branches via
    // 0x4817a directly to 0x48212, bypassing the service switch check on the
    // 7th service1 press at 2020.
    assert_eq!(
        service_debounce_frames,
        6 * 6,
        "six 6-frame service debounce sleeps at 0x481e8"
    );
    assert_eq!(
        service_cooldown_frames,
        6 * 30,
        "six 30-frame service cooldown sleeps at 0x48212"
    );

    // At 9 credits (awarded by the 6th service1 press at 1973, cooldown ends
    // at 2003, loop sleeps 6 frames to 2009, next pass at 2010), 0x4817a
    // calls 0x3b6a2(0x7070) to clear 0xff8684 and write 0xc00010 = 0x0000.
    assert_eq!(
        lockout_frame,
        Some((2010, 9)),
        "9-credit lockout engages at frame 2010 with 9 credits"
    );
    assert_eq!(
        slot1_debounce_at_2085,
        Some(0),
        "coin1 at 2080..2092 is suppressed at IN0 by coin_ctrl lockout"
    );
    assert_eq!(
        credits_at_2095,
        Some(9),
        "credits remain capped at 9 after locked-out coin1 at 2080"
    );
}

/// Criterion 5:
/// - 1P start (`sf-credit-select-01` at 1521, `sf-credits-01` at 2141) gives
///   mode `2` and takes 1 credit (`1 -> 0` and `9 -> 8`).
/// - 2P start with 1 credit (`sf-credits-01` at `1541..=1552`) is ignored.
/// - 2P start with 2 credits (`sf-start-2p-01` at 1561) gives mode `1` and
///   takes 2 credits (`2 -> 0`).
/// - The exit erases `0x3e038`'s five blocks even on a 1-credit start.
/// - `0x3e04c` is reported (`C-STOP`).
#[test]
fn start_presses_deduct_credits_set_mode_erase_five_blocks_and_stop_at_0x3e04c() {
    let Some(Captures {
        assets,
        cs_a,
        cr_a,
        s2_a,
        ..
    }) = require_captures()
    else {
        eprintln!("skipping: start-screen reference captures not present");
        return;
    };
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    // 1. 2P start with 1 credit in sf-credits-01 (frames 1541..=1552) is ignored.
    {
        let scenario = manifest_dir.join(CREDITS_SCENARIO);
        let outcome =
            compare_cpu_ram_with_inputs(&cr_a, assets.maincpu.clone(), Some(&scenario), 1500, 1560)
                .unwrap();
        assert_eq!(outcome, CpuRamOutcome::Match { frames: 60 });
        // Step through 2141 (1 frame past 2140, where start1 is pressed with 9
        // credits, entering two-country select at 0x3e04c):
        let outcome_2141 =
            compare_cpu_ram_with_inputs(&cr_a, assets.maincpu.clone(), Some(&scenario), 1500, 2141)
                .unwrap();
        assert_eq!(
            outcome_2141,
            CpuRamOutcome::Match { frames: 641 },
            "sf-credits-01 matches through country-select entry at frame 2141"
        );
        // Step a machine from checkpoint 2140 with start1 held and 0xffc005 = 0
        // (4-country DIP) to verify 0x3e038 erased credit digits and the
        // start-screen prompt blocks before 0x3e056 branches to 0x3e1e4:
        let mut m = CpuMachine::from_fight_checkpoint(&cr_a, 2140, assets.maincpu.clone()).unwrap();
        m.set_work_byte(0xffc005, 0);
        m.set_cabinet_inputs(false, false, true, false, false);
        m.step();
        assert!(m.unrecovered().is_empty());
        let w = m.work_ram();
        assert!(
            m.task_positions().contains(&(1, 0x3e252)),
            "0xffc005 == 0 branches through 0x3e1e4 and sleeps at 0x3e252"
        );
        assert_eq!(w[0x4002], 2, "1P start sets mode (0xffc002) = 2");
        assert_eq!(
            u16::from_be_bytes([w[0x065c], w[0x065d]]),
            8,
            "1P start from 9 credits deducts 1 credit to 8"
        );
        let off_2p =
            u16::from_be_bytes([assets.maincpu[0x14e4e], assets.maincpu[0x14e4f]]) as usize;
        let cols_2p = assets.maincpu[0x14e4e + 3] as usize;
        let last_cell = off_2p + 2 * (cols_2p - 1);
        assert_eq!(
            &m.state.videoram[0x0058..0x005c],
            &[0x10, 0x3c, 0x10, 0x3c],
            "exit erases credit digits at 0xff0058..0xff005c"
        );
        assert_eq!(
            &m.state.videoram[last_cell..last_cell + 2],
            &[0x10, 0x3c],
            "exit erases 2-credit prompt block 0x14e4e"
        );
    }

    // 2. 1P start with 1 credit in sf-credit-select-01 (frame 1521):
    // mode = 2, credits 1 -> 0, erases all 5 text blocks (including the 2P
    // prompt block at 0xff0906 even though only 1 credit was present), and
    // enters two-country select at 0x3e04c (or four-country select at 0x3e1e4
    // when 0xffc005 == 0).
    {
        let scenario = manifest_dir.join(CREDIT_SELECT_SCENARIO);
        let outcome =
            compare_cpu_ram_with_inputs(&cs_a, assets.maincpu.clone(), Some(&scenario), 1500, 1521)
                .unwrap();
        assert_eq!(
            outcome,
            CpuRamOutcome::Match { frames: 21 },
            "sf-credit-select-01 matches through country-select entry on frame 1521"
        );
        let mut m = CpuMachine::from_fight_checkpoint(&cs_a, 1520, assets.maincpu.clone()).unwrap();
        // Dirty the 2-credit prompt block (0x14e4e, the 3rd pointer in 0x3e038,
        // which is wider than the 1-credit prompt 0x14e2a) before stepping
        // frame 1521 to prove 0x3e038 erases all 5 blocks even on a 1-credit
        // start.
        let off_2p =
            u16::from_be_bytes([assets.maincpu[0x14e4e], assets.maincpu[0x14e4f]]) as usize;
        let cols_2p = assets.maincpu[0x14e4e + 3] as usize;
        let last_cell = off_2p + 2 * (cols_2p - 1);
        m.state.videoram[last_cell] = 0x12;
        m.state.videoram[last_cell + 1] = 0x34;
        m.set_work_byte(0xffc005, 0);
        m.set_cabinet_inputs(false, false, true, false, false);
        m.step();
        assert!(m.unrecovered().is_empty());
        let w = m.work_ram();
        assert!(
            m.task_positions().contains(&(1, 0x3e252)),
            "0xffc005 == 0 branches through 0x3e1e4 and sleeps at 0x3e252"
        );
        assert_eq!(w[0x4002], 2, "1P start sets mode (0xffc002) = 2");
        assert_eq!(
            u16::from_be_bytes([w[0x065c], w[0x065d]]),
            0,
            "1P start from 1 credit deducts 1 credit to 0"
        );
        assert_eq!(
            &m.state.videoram[last_cell..last_cell + 2],
            &[0x10, 0x3c],
            "1-credit start erases all 5 blocks of 0x3e038 (including 0x14e4e)"
        );
    }

    // 3. 2P start with 2 credits in sf-start-2p-01 (frame 1561):
    // mode = 1, credits 2 -> 0, erases all 5 blocks, and enters two-country
    // select at 0x3e04c (or four-country select at 0x3e1e4 when 0xffc005 == 0).
    {
        let scenario = manifest_dir.join(START_2P_SCENARIO);
        let outcome =
            compare_cpu_ram_with_inputs(&s2_a, assets.maincpu.clone(), Some(&scenario), 1500, 1561)
                .unwrap();
        assert_eq!(
            outcome,
            CpuRamOutcome::Match { frames: 61 },
            "sf-start-2p-01 matches through country-select entry on frame 1561"
        );
        let mut m = CpuMachine::from_fight_checkpoint(&s2_a, 1560, assets.maincpu.clone()).unwrap();
        m.set_work_byte(0xffc005, 0);
        m.set_cabinet_inputs(false, false, false, true, false);
        m.step();
        assert!(m.unrecovered().is_empty());
        let w = m.work_ram();
        assert!(
            m.task_positions().contains(&(1, 0x3e252)),
            "0xffc005 == 0 branches through 0x3e1e4 and sleeps at 0x3e252"
        );
        assert_eq!(w[0x4002], 1, "2P start sets mode (0xffc002) = 1");
        assert_eq!(
            u16::from_be_bytes([w[0x065c], w[0x065d]]),
            0,
            "2P start from 2 credits deducts 2 credits to 0"
        );
        let off_2p =
            u16::from_be_bytes([assets.maincpu[0x14e4e], assets.maincpu[0x14e4f]]) as usize;
        let cols_2p = assets.maincpu[0x14e4e + 3] as usize;
        let last_cell = off_2p + 2 * (cols_2p - 1);
        assert_eq!(
            &m.state.videoram[0x0058..0x005c],
            &[0x10, 0x3c, 0x10, 0x3c],
            "2P start erases credit digits at 0xff0058..0xff005c"
        );
        assert_eq!(
            &m.state.videoram[last_cell..last_cell + 2],
            &[0x10, 0x3c],
            "2P start erases 2-credit prompt block 0x14e4e at 1561"
        );
    }
}

/// Criterion 6 (`C-NEG`): delaying a coin press by 10 frames (`coin2` at
/// `1590` instead of `1580` in `sf-credits-01`) is caught by both
/// `compare_cpu_ram_with_inputs` and `sf_compare::compare`.
#[test]
fn delaying_a_coin_press_by_ten_frames_is_caught_by_comparison() {
    let Some(Captures { assets, cr_a, .. }) = require_captures() else {
        eprintln!("skipping: start-screen reference captures not present");
        return;
    };
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let orig_text = std::fs::read_to_string(manifest_dir.join(CREDITS_SCENARIO)).unwrap();
    let delayed_text = orig_text.replace(r#""frame": 1580"#, r#""frame": 1590"#);
    assert_ne!(orig_text, delayed_text, "coin2 at 1580 replaced");

    let scratch = ScratchDir::new("sf-credits-01-neg");
    std::fs::create_dir_all(&*scratch).unwrap();
    let delayed_scenario = scratch.join("delayed.json");
    std::fs::write(&delayed_scenario, delayed_text).unwrap();

    // 1. Caught by owned work-RAM comparison at frame 1581 (when reference
    // sees coin2 debounce 0xff8642 increment to 1).
    let ram_outcome = compare_cpu_ram_with_inputs(
        &cr_a,
        assets.maincpu.clone(),
        Some(&delayed_scenario),
        1500,
        1600,
    )
    .unwrap();
    match ram_outcome {
        CpuRamOutcome::Mismatch(msg) => {
            assert!(
                msg.contains("frame 1581") && msg.contains("0xff8642"),
                "expected mismatch at frame 1581 on coin2 debounce 0xff8642, got: {msg}"
            );
        }
        other => panic!("expected CpuRamOutcome::Mismatch, got: {other:?}"),
    }

    // 2. Caught by capture comparison (`sf_compare::compare` on screen_hash,
    // latch, pixels, video RAM).
    let out_capture = scratch.join("capture");
    run_cpu_fight_capture_with_inputs(
        &cr_a,
        &assets,
        &out_capture,
        "sf-credits-01",
        Some(&delayed_scenario),
        1500,
        1600,
    )
    .unwrap();
    let scenario = load_scenario(&delayed_scenario).unwrap();
    let ref_cap = load_capture(&cr_a).unwrap();
    let cand_cap = load_capture(&out_capture).unwrap();
    let scope = CompareScope::default().with_range(1501, 1600).with_fields(
        CompareScope::parse_fields("screen_hash,latch,latch_changed")
            .expect("valid observable field names"),
    );
    let outcome = compare(
        &scenario,
        &ref_cap,
        &cand_cap,
        CandidateKind::Native,
        &scope,
    )
    .unwrap();
    assert!(
        matches!(outcome, Outcome::Mismatch(_)),
        "expected sf_compare::compare to reject delayed coin2 press, got: {outcome:?}"
    );
}

/// Ticket #73 (`F33`): a coin during the title opens the start screen.
/// Verifies `C-NAT` from `sf-credit-select-01`'s `1440` fixture through `1521`
/// (and `1440..=2140` on `sf-credits-01`, `1440..=1560` on `sf-start-2p-01`),
/// all frame landmarks (`1450..=1462` debounce, `1467` slot-1 re-creation at
/// `0x3dec2 -> 0x3dfce` and `0xffc007 == 0xff8f28`, `0x0f` / `0x3f` / `0x80`
/// latches at `1468` / `1469` / `1470`, and 1P start at `1521` stopping at
/// `0x3e04c`), and that `0xff8f28` is in the owned RAM lists.
#[test]
fn title_coin_opens_start_screen_and_matches_from_1440_through_1521() {
    let Some(Captures {
        assets,
        cs_a,
        cr_a,
        s2_a,
        ..
    }) = require_captures()
    else {
        eprintln!("skipping: start-screen reference captures not present");
        return;
    };
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cs_scenario = manifest_dir.join(CREDIT_SELECT_SCENARIO);

    // 1. Owned RAM lists include 0xff8f28.
    assert!(
        sf_engine::run::CPU_RAM_REGIONS
            .iter()
            .any(|&(_, start, end)| (start..end).contains(&0xff8f28)),
        "0xff8f28 must be in CPU_RAM_REGIONS"
    );
    assert!(
        sf_engine::run::VERSUS_RAM_REGIONS
            .iter()
            .any(|&(_, start, end)| (start..end).contains(&0xff8f28)),
        "0xff8f28 must be in VERSUS_RAM_REGIONS"
    );

    // 2. Full C-NAT comparison from 1440 through the start press across all
    // three start-screen captures.
    verify_native_interval(
        &assets,
        &cs_a,
        &cs_scenario,
        "sf-credit-select-01",
        1440,
        1520,
    );
    verify_native_interval(
        &assets,
        &cr_a,
        &manifest_dir.join(CREDITS_SCENARIO),
        "sf-credits-01",
        1440,
        2140,
    );
    verify_native_interval(
        &assets,
        &s2_a,
        &manifest_dir.join(START_2P_SCENARIO),
        "sf-start-2p-01",
        1440,
        1560,
    );

    // 3. Verify landmarks over 1440..=1520 and the stop at 1521 on sf-credit-select-01.
    let mut latches = Vec::new();
    let mut prev_latch = 0xffu8;
    let (stopped, unrecovered) = observe_cpu_fight(
        &cs_a,
        assets.maincpu.clone(),
        Some(&cs_scenario),
        1440,
        1520,
        |m| {
            let f = m.state.frame;
            let w = m.work_ram();
            let ref_work = std::fs::read(cs_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
            let pos = Scheduler::from_fixture(&ref_work).unwrap().positions();
            if (1451..=1462).contains(&f) {
                assert_eq!(
                    w[0x063a],
                    (f - 1450) as u8,
                    "frame {f}: coin1 debounce counter 0xff863a"
                );
                assert_eq!(w[0x063b], 0, "frame {f}: coin1 pending still 0");
            }
            if f == 1463 {
                assert_eq!(w[0x063a], 0, "frame 1463: coin1 release clears debounce");
                assert_eq!(w[0x063b], 1, "frame 1463: coin1 release increments pending");
            }
            if f == 1466 {
                assert!(
                    pos.contains(&(1, 0x3b588)) && pos.contains(&(7, 0x4819e)),
                    "frame 1466 positions: {pos:05x?}"
                );
            }
            if f == 1467 {
                // Slot 7 copied 0xff8f28 -> 0xffc007, killed slot 2, spawned
                // slot 1 at 0x3dec2 (which ran in the same frame under the
                // scheduler rescan and slept at 0x3dfce), and slept at 0x4824e.
                assert_eq!(w[0x0003], 0xff, "frame 1467: MODE_DEMO set to 0xff");
                assert_eq!(w[0x4007], w[0x0f28], "frame 1467: 0xffc007 == 0xff8f28");
                assert_eq!(w[0x0026], 0, "frame 1467: slot 2 killed");
                assert!(
                    pos.contains(&(1, 0x3dfce)) && pos.contains(&(7, 0x4824e)),
                    "frame 1467 positions: {pos:05x?}"
                );
            }
            if f == 1468 {
                assert!(
                    pos.contains(&(1, 0x3dfce)) && pos.contains(&(7, 0x48344)),
                    "frame 1468 positions: {pos:05x?}"
                );
            }
            if m.state.sound_latch != prev_latch {
                latches.push((f, m.state.sound_latch));
                prev_latch = m.state.sound_latch;
            }
        },
    )
    .unwrap();
    assert_eq!((stopped, unrecovered), (1520, Vec::new()));
    assert_eq!(
        latches,
        vec![(1468, 0x0f), (1469, 0x3f), (1470, 0x80), (1471, 0xff)],
        "expected 0x0f at 1468, 0x3f at 1469, 0x80 at 1470, 0xff at 1471"
    );

    // Continuing through 1521 on 1P start from 1440:
    let match_outcome = compare_cpu_ram_with_inputs(
        &cs_a,
        assets.maincpu.clone(),
        Some(&cs_scenario),
        1440,
        1521,
    )
    .unwrap();
    assert_eq!(match_outcome, CpuRamOutcome::Match { frames: 81 });

    // Verify 0x48228 copies a non-zero 0xff8f28 into 0xffc007 on attract start:
    let mut m = CpuMachine::from_fight_checkpoint(&cs_a, 1466, assets.maincpu.clone()).unwrap();
    m.set_work_byte(0xff8f28, 3);
    m.set_work_byte(0xffc007, 0);
    m.step();
    assert_eq!(
        m.work_ram()[0x4007],
        3,
        "0x48228 copies non-zero 0xff8f28 (3) to 0xffc007"
    );
}

/// Ticket #73 (`C-NEG`): delaying the title coin by 10 frames (`coin1` at
/// `1460` instead of `1450` in `sf-credit-select-01`) when replaying from
/// `1440` is caught by both `compare_cpu_ram_with_inputs` and
/// `sf_compare::compare`.
#[test]
fn delaying_title_coin_by_ten_frames_is_caught_from_1440() {
    let Some(Captures { assets, cs_a, .. }) = require_captures() else {
        eprintln!("skipping: start-screen reference captures not present");
        return;
    };
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let orig_text = std::fs::read_to_string(manifest_dir.join(CREDIT_SELECT_SCENARIO)).unwrap();
    let delayed_text = orig_text.replace(r#""frame": 1450"#, r#""frame": 1460"#);
    assert_ne!(orig_text, delayed_text, "coin1 at 1450 replaced");

    let scratch = ScratchDir::new("sf-credit-select-01-neg-1440");
    std::fs::create_dir_all(&*scratch).unwrap();
    let delayed_scenario = scratch.join("delayed_1440.json");
    std::fs::write(&delayed_scenario, delayed_text).unwrap();

    let ram_outcome = compare_cpu_ram_with_inputs(
        &cs_a,
        assets.maincpu.clone(),
        Some(&delayed_scenario),
        1440,
        1500,
    )
    .unwrap();
    match ram_outcome {
        CpuRamOutcome::Mismatch(msg) => {
            assert!(
                msg.contains("frame 1451") && msg.contains("0xff863a"),
                "expected mismatch at frame 1451 on coin1 debounce 0xff863a, got: {msg}"
            );
        }
        other => panic!("expected CpuRamOutcome::Mismatch, got: {other:?}"),
    }

    let out_capture = scratch.join("capture");
    run_cpu_fight_capture_with_inputs(
        &cs_a,
        &assets,
        &out_capture,
        "sf-credit-select-01",
        Some(&delayed_scenario),
        1440,
        1500,
    )
    .unwrap();
    let scenario = load_scenario(&delayed_scenario).unwrap();
    let ref_cap = load_capture(&cs_a).unwrap();
    let cand_cap = load_capture(&out_capture).unwrap();
    let scope = CompareScope::default().with_range(1441, 1500).with_fields(
        CompareScope::parse_fields("screen_hash,latch,latch_changed")
            .expect("valid observable field names"),
    );
    let outcome = compare(
        &scenario,
        &ref_cap,
        &cand_cap,
        CandidateKind::Native,
        &scope,
    )
    .unwrap();
    assert!(
        matches!(outcome, Outcome::Mismatch(_)),
        "expected sf_compare::compare to reject delayed title coin1 press, got: {outcome:?}"
    );
}

/// Ticket #75 (`F35`): Coinage DIP — coins at non-default rates (`sf-coinage-01`,
/// `Coin A = 2C_1C`, `Coin B = 1C_6C`, `dsw1 = 0xdfda`, `dsw2 = 0xffff`).
/// Verifies:
/// 1. `C-REF`: `coinage-01-run-a == coinage-01-run-b`, and `coin2@1450` awards
///    6 credits and opens the start screen in the reference (and natively from
///    `1440` and `1500` through `1720`).
/// 2. `C-NAT` from checkpoint `1500` (and `1440`) through `1720`:
///    - first `coin1@1540` gives 0 credits (`6` total, `0xff8641` partial-coin
///      counter decrements from `2` to `1`);
///    - second `coin1@1600` gives 1 credit (`7` total, `0xff8641` reloads `2`);
///    - second `coin2@1660` awards 2 credits (`7 -> 9`), stops at `9` inside
///      `0x48308`, and `0x3b6a2` clears `0x7070` (`0xff8684 = 0x0000`).
/// 3. Static check decoding all 64 `(Coin A, Coin B)` settings through
///    `0x1456a` with `0x3b2e4` (`boot_dip_decode_0x3b286`).
#[test]
fn coinage_dip_non_default_rates_and_all_64_coinage_settings_match_reference() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Some(roms) = local(ROMS) else {
        eprintln!("skipping: ROMs not present");
        return;
    };
    let manifest = manifest_dir.join(MANIFEST);
    let assets = load_assets(&roms, &manifest).expect("ROM set assembles");

    // Criterion 3: static check decoding all 64 (Coin A, Coin B) settings
    // through 0x1456a with 0x3b2e4.
    const EXPECTED_COINAGE: [(u8, u8); 8] = [
        (1, 1), // 1C_1C (inverted index 0)
        (1, 2), // 1C_2C (inverted index 1)
        (1, 3), // 1C_3C (inverted index 2)
        (1, 4), // 1C_4C (inverted index 3)
        (1, 6), // 1C_6C (inverted index 4)
        (2, 1), // 2C_1C (inverted index 5)
        (3, 1), // 3C_1C (inverted index 6)
        (4, 1), // 4C_1C (inverted index 7)
    ];
    for inv_a in 0u16..8 {
        for inv_b in 0u16..8 {
            let dsw1 = !(inv_a | (inv_b << 3)) & 0xdfff;
            let mut m = sf_engine::run::PowerOnMachine::with_io(
                assets.maincpu.clone(),
                sf_engine::tasks::Io {
                    dsw1,
                    dsw2: 0xffff,
                    ..sf_engine::tasks::Io::idle()
                },
            );
            m.step();
            let w = m.work_ram();
            let (coins_a, creds_a) = EXPECTED_COINAGE[inv_a as usize];
            let (coins_b, creds_b) = EXPECTED_COINAGE[inv_b as usize];
            // Slot 1 (Coin A, 0xff863a): +2 (0xff863c) = coins_needed, +3 (0xff863d) = credits_per_threshold
            assert_eq!(w[0x063c], coins_a, "inv_a={inv_a}: 0xff863c coins_needed");
            assert_eq!(
                w[0x063d], creds_a,
                "inv_a={inv_a}: 0xff863d credits_per_threshold"
            );
            // Slot 2 (Coin B, 0xff8642): +2 (0xff8644) = coins_needed, +3 (0xff8645) = credits_per_threshold
            assert_eq!(w[0x0644], coins_b, "inv_b={inv_b}: 0xff8644 coins_needed");
            assert_eq!(
                w[0x0645], creds_b,
                "inv_b={inv_b}: 0xff8645 credits_per_threshold"
            );
        }
    }

    let (Some(coinage_a), Some(coinage_b)) = (
        local("../../tools/reference/local/captures/coinage-01-run-a"),
        local("../../tools/reference/local/captures/coinage-01-run-b"),
    ) else {
        eprintln!("skipping: coinage-01 reference captures not present");
        return;
    };
    let scenario_path = manifest_dir.join("../../tools/reference/scenarios/sf_coinage_01.json");

    // Criterion 1 (C-REF): coinage-01-run-a == coinage-01-run-b, dsw1 = 0xdfda, dsw2 = 0xffff.
    let scenario = load_scenario(&scenario_path).expect("load sf_coinage_01.json");
    assert_eq!((scenario.dsw1, scenario.dsw2), (0xdfda, 0xffff));
    assert_dir_identical(&coinage_a, &coinage_b, "sf-coinage-01");

    // Criterion 2 (C-NAT): verify full native intervals from both 1500 and 1440 through 1720.
    verify_native_interval(
        &assets,
        &coinage_a,
        &scenario_path,
        "sf-coinage-01",
        1500,
        1720,
    );
    verify_native_interval(
        &assets,
        &coinage_a,
        &scenario_path,
        "sf-coinage-01",
        1440,
        1720,
    );

    // Observe the exact credit and partial-coin transitions across 1440..=1720:
    let mut credits_at_1500 = None;
    let mut after_first_coin1 = None;
    let mut after_second_coin1 = None;
    let mut after_second_coin2 = None;
    let (stopped, unrecovered) = observe_cpu_fight(
        &coinage_a,
        assets.maincpu.clone(),
        Some(&scenario_path),
        1440,
        1720,
        |m| {
            let f = m.state.frame;
            let w = m.work_ram();
            let credits = u16::from_be_bytes([w[0x065c], w[0x065d]]);
            let partial_a = w[0x063e];
            let coin_ctrl = u16::from_be_bytes([w[0x0684], w[0x0685]]);
            match f {
                1500 => credits_at_1500 = Some((credits, partial_a, coin_ctrl)),
                1570 => after_first_coin1 = Some((credits, partial_a, coin_ctrl)),
                1630 => after_second_coin1 = Some((credits, partial_a, coin_ctrl)),
                1700 => after_second_coin2 = Some((credits, partial_a, coin_ctrl)),
                _ => {}
            }
        },
    )
    .unwrap();
    assert_eq!((stopped, unrecovered), (1720, Vec::new()));
    assert_eq!(
        credits_at_1500,
        Some((6, 0, 0x7070)),
        "at 1500: 6 credits from coin2@1450 (1C_6C), slot 1 partial_count (0xff863e) = 0"
    );
    assert_eq!(
        after_first_coin1,
        Some((6, 1, 0x7070)),
        "after first coin1@1540 (2C_1C): 0 credits awarded (still 6), partial_count (0xff863e) = 1"
    );
    assert_eq!(
        after_second_coin1,
        Some((7, 0, 0x7070)),
        "after second coin1@1600 (2C_1C): 1 credit awarded (7 total), partial_count (0xff863e) reset to 0"
    );
    assert_eq!(
        after_second_coin2,
        Some((9, 0, 0x0000)),
        "after second coin2@1660 (1C_6C): 2 credits awarded (7 -> 9), capped at 9 at 0x48308, 0x3b6a2 clears 0x7070"
    );
}

/// Ticket #74 (`F34`): Two-country select — timeout to Japan (`sf-credit-select-01`
/// and `sf-start-2p-01`), stick and button to USA (`sf-select-usa-01`), the 15
/// opponent pointer arrays at `0xffc01c..0xffc057` (`0x19908 + 0x28 * country +
/// 0xa0 * n`), records (`ai-tables.md`, `round-timer.md`, `sf_fixed_fight_01.json`),
/// and `C-NEG` (LP 10 frames late).
#[test]
fn two_country_select_timeout_and_usa_button_and_opponent_arrays_match_reference() {
    let Some(Captures {
        assets,
        cs_a,
        s2_a,
        fight_a,
        ..
    }) = require_captures()
    else {
        eprintln!("skipping: start-screen reference captures not present");
        return;
    };
    let (Some(usa_a), Some(usa_b), Some(cpu_a)) = (
        local("../../tools/reference/local/captures/select-usa-01-run-a"),
        local("../../tools/reference/local/captures/select-usa-01-run-b"),
        local("../../tools/reference/local/captures/cpu-01-run-a"),
    ) else {
        eprintln!("skipping: select-usa-01 or cpu-01 reference captures not present");
        return;
    };
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cs_scenario = manifest_dir.join(CREDIT_SELECT_SCENARIO);
    let s2_scenario = manifest_dir.join(START_2P_SCENARIO);
    let usa_scenario = manifest_dir.join("../../tools/reference/scenarios/sf_select_usa_01.json");

    // Criterion 2: 0xffc01c..0xffc057 are in CPU_RAM_REGIONS and VERSUS_RAM_REGIONS,
    // and static check confirms 0x19908 + 0x28 * country + 0xa0 * n across all
    // 4 countries (0..4) and 15 arrays (0..15).
    for addr in 0xffc01cu32..0xffc058 {
        assert!(
            sf_engine::run::CPU_RAM_REGIONS
                .iter()
                .any(|&(_, start, end)| (start..end).contains(&addr)),
            "0x{addr:06x} must be in CPU_RAM_REGIONS"
        );
        assert!(
            sf_engine::run::FIGHT_RAM_REGIONS
                .iter()
                .any(|&(_, start, end)| (start..end).contains(&addr)),
            "0x{addr:06x} must be in FIGHT_RAM_REGIONS"
        );
        if addr < 0xffc050 {
            assert!(
                sf_engine::run::VERSUS_RAM_REGIONS
                    .iter()
                    .any(|&(_, start, end)| (start..end).contains(&addr)),
                "0x{addr:06x} must be in VERSUS_RAM_REGIONS"
            );
        }
    }
    const HANDLERS: [usize; 4] = [0x3e39a, 0x3e416, 0x3e492, 0x3e50e];
    for (country, &handler) in HANDLERS.iter().enumerate() {
        let table_entry = u32::from_be_bytes([
            assets.maincpu[0x3e38a + 4 * country],
            assets.maincpu[0x3e38a + 4 * country + 1],
            assets.maincpu[0x3e38a + 4 * country + 2],
            assets.maincpu[0x3e38a + 4 * country + 3],
        ]) as usize;
        assert_eq!(table_entry, handler, "0x3e38a[{country}]");
        for n in 0usize..15 {
            let insn = handler + 8 * n;
            let op = u16::from_be_bytes([assets.maincpu[insn], assets.maincpu[insn + 1]]);
            let imm = u32::from_be_bytes([
                assets.maincpu[insn + 2],
                assets.maincpu[insn + 3],
                assets.maincpu[insn + 4],
                assets.maincpu[insn + 5],
            ]);
            let disp = u16::from_be_bytes([assets.maincpu[insn + 6], assets.maincpu[insn + 7]]);
            assert_eq!(op, 0x2b7c, "country {country} n={n}: move.l #imm, (d16,A5)");
            assert_eq!(
                disp,
                0x401c + 4 * n as u16,
                "country {country} n={n}: dest disp"
            );
            assert_eq!(
                imm,
                0x19908 + 0x28 * country as u32 + 0xa0 * n as u32,
                "country {country} n={n}: expected 0x19908 + 0x28*{country} + 0xa0*{n}"
            );
        }
    }

    // Criterion 1 & 2: C-NAT from 1500 (and 1440) over 1501..=1866 (and through
    // 1870 into F4's StageStart) on sf-credit-select-01.
    verify_native_interval(
        &assets,
        &cs_a,
        &cs_scenario,
        "sf-credit-select-01",
        1500,
        1870,
    );
    verify_native_interval(
        &assets,
        &cs_a,
        &cs_scenario,
        "sf-credit-select-01",
        1440,
        1870,
    );

    let mut latches_cs = Vec::new();
    let mut prev_latch = 0xffu8;
    let mut fg_scroll_1521 = None;
    let mut pal_req_1521 = None;
    let mut tally4_1523 = None;
    let mut tally4_1865 = None;
    let mut exit_state_1865 = None;
    let mut arrays_1866 = None;
    let (stopped, unrecovered) = observe_cpu_fight(
        &cs_a,
        assets.maincpu.clone(),
        Some(&cs_scenario),
        1500,
        1870,
        |m| {
            let f = m.state.frame;
            let w = m.work_ram();
            if m.state.sound_latch != prev_latch {
                latches_cs.push((f, m.state.sound_latch));
                prev_latch = m.state.sound_latch;
            }
            match f {
                1521 => {
                    fg_scroll_1521 = Some((
                        m.state.fg_scroll,
                        u16::from_be_bytes([w[0x4680], w[0x4681]]),
                    ));
                    let src = u32::from_be_bytes([w[0x0634], w[0x0635], w[0x0636], w[0x0637]]);
                    let off = u16::from_be_bytes([w[0x0638], w[0x0639]]);
                    pal_req_1521 = Some((w[0x0633], src, off));
                }
                1523 => {
                    tally4_1523 = Some(w[0x0e30..0x0e36].to_vec());
                }
                1865 => {
                    tally4_1865 = Some(w[0x0e30..0x0e36].to_vec());
                    // Slot 2 spawned at 0x3f7b0 and ran in frame 1865, sleeping
                    // (state 8, sleep 1) at 0x3f7d6; slot 1 sleeps at 0x3e59c.
                    exit_state_1865 = Some((w[0x4006], w[0x0016], w[0x0017], w[0x0026], w[0x0027]));
                }
                1866 => {
                    arrays_1866 = Some(w[0x401c..0x4058].to_vec());
                }
                _ => {}
            }
        },
    )
    .unwrap();
    assert_eq!((stopped, unrecovered), (1870, Vec::new()));
    assert_eq!(
        fg_scroll_1521,
        Some((0x4200, 0x4200)),
        "fg_scroll = 0x4200 at 1521"
    );
    assert_eq!(
        pal_req_1521,
        Some((0xff, 0x1a468, 0x200)),
        "palette 0x1a468 at 0x200 requested at 1521"
    );
    assert_eq!(
        tally4_1523,
        Some(vec![0x00, 0x05, 0x00, 0x00, 0x02, 0x00]),
        "tally 4 reloaded to 05.00 at 1523"
    );
    assert_eq!(
        tally4_1865,
        Some(vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x01]),
        "tally 4 expires with done=1 at 1865"
    );
    assert_eq!(
        exit_state_1865,
        Some((0, 8, 1, 8, 1)),
        "at 1865: country=0, slot 1 sleeping (8,1) at 0x3e59c, slot 2 spawned and ran to sleep (8,1) at 0x3f7d6"
    );
    assert_eq!(
        latches_cs,
        vec![
            (1522, 0x34),
            (1523, 0xff),
            (1866, 0x3f),
            (1867, 0xff),
            (1868, 0x8f),
            (1869, 0xff),
        ],
        "0x34 at 1522, 0x3f at 1866, and 0x8f (event 3 Japan plane sound) at 1868"
    );

    // Verify 0xffc01c..0xffc057 at 1866 equal the reference at 1866, 1980 (fight-run-a)
    // and 2389 (cpu-01-run-a):
    let arrays_1866 = arrays_1866.expect("arrays captured at 1866");
    let ref_1866 = std::fs::read(cs_a.join("ram_001866_work.bin")).unwrap();
    let ref_1980 = std::fs::read(fight_a.join("ram_001980_work.bin")).unwrap();
    let ref_2389 = std::fs::read(cpu_a.join("ram_002389_work.bin")).unwrap();
    assert_eq!(
        &arrays_1866[..],
        &ref_1866[0x401c..0x4058],
        "native vs ref at 1866"
    );
    assert_eq!(
        &arrays_1866[..],
        &ref_1980[0x401c..0x4058],
        "unchanged at 1980 in fight-run-a"
    );
    assert_eq!(
        &arrays_1866[..],
        &ref_2389[0x401c..0x4058],
        "unchanged at 2389 in sf-cpu-01"
    );

    // Criterion 3: C-REF and C-NAT on sf-select-usa-01 (right, left, right,
    // diagonal, LP), with checkpoints every frame 1440..=1670.
    let usa_scen = load_scenario(&usa_scenario).unwrap();
    assert_eq!(usa_scen.checkpoints, (1440u64..=1670).collect::<Vec<_>>());
    assert_dir_identical(&usa_a, &usa_b, "sf-select-usa-01");
    assert_frames_prefix_identical(
        &usa_a,
        &cs_a,
        1539,
        "sf-select-usa-01 vs sf-credit-select-01",
    );

    verify_native_interval(
        &assets,
        &usa_a,
        &usa_scenario,
        "sf-select-usa-01",
        1500,
        1670,
    );
    verify_native_interval(
        &assets,
        &usa_a,
        &usa_scenario,
        "sf-select-usa-01",
        1440,
        1670,
    );

    let mut country_trace = Vec::new();
    let mut arrays_usa = None;
    let mut latch_1662 = None;
    let (stopped_usa, unrec_usa) = observe_cpu_fight(
        &usa_a,
        assets.maincpu.clone(),
        Some(&usa_scenario),
        1500,
        1670,
        |m| {
            let f = m.state.frame;
            let w = m.work_ram();
            if matches!(
                f,
                1540 | 1541 | 1570 | 1571 | 1600 | 1601 | 1631 | 1642 | 1660 | 1661
            ) {
                country_trace.push((f, w[0x4006], w[0x068f]));
            }
            if f == 1661 {
                arrays_usa = Some(w[0x401c..0x4058].to_vec());
            }
            if f == 1662 {
                latch_1662 = Some(m.state.sound_latch);
            }
        },
    )
    .unwrap();
    assert_eq!((stopped_usa, unrec_usa), (1670, Vec::new()));
    assert_eq!(
        country_trace,
        vec![
            (1540, 0, 0xff), // before right@1540: Japan (0)
            (1541, 2, 0xfe), // pure right (0xfe) -> USA (2)
            (1570, 2, 0xff), // before left@1570: USA (2)
            (1571, 0, 0xfd), // pure left (0xfd) -> Japan (0)
            (1600, 0, 0xff), // before right@1600: Japan (0)
            (1601, 2, 0xfe), // pure right (0xfe) -> USA (2)
            (1631, 2, 0xf5), // up-left diagonal (0xf5) -> ignored, stays USA (2)
            (1642, 2, 0xf5), // last frame of diagonal -> still USA (2)
            (1660, 2, 0xef), // LP held (IN1 bit 4 low = 0xef)
            (1661, 2, 0xff), // LP released -> button latch 0xff86b0=0x4000 exits with USA (2, bit 1 set)
        ]
    );
    assert_eq!(
        latch_1662,
        Some(0x3f),
        "0x3f popped at 1662 after LP exit at 1661"
    );
    let arrays_usa = arrays_usa.expect("USA arrays captured at 1661");
    for n in 0usize..15 {
        let ptr = u32::from_be_bytes([
            arrays_usa[4 * n],
            arrays_usa[4 * n + 1],
            arrays_usa[4 * n + 2],
            arrays_usa[4 * n + 3],
        ]);
        assert_eq!(
            ptr,
            0x19958 + 0xa0 * n as u32,
            "USA array {n}: starts at 0x19958 + 0xa0*{n}"
        );
    }

    // Criterion 4: sf-start-2p-01 continued through its select in mode 1
    // (2P start at 1561 -> timeout exit at 1905, 0x3f at 1906) matches.
    verify_native_interval(&assets, &s2_a, &s2_scenario, "sf-start-2p-01", 1500, 1906);
    verify_native_interval(&assets, &s2_a, &s2_scenario, "sf-start-2p-01", 1440, 1906);

    // Criterion 5: records corrected in ai-tables.md, round-timer.md, and
    // sf_fixed_fight_01.json.
    let ai_doc = std::fs::read_to_string(manifest_dir.join("../../docs/reference/ai-tables.md"))
        .expect("read ai-tables.md");
    assert!(
        ai_doc.contains("15 per-opponent pointer") && !ai_doc.contains("13 per-opponent pointer"),
        "ai-tables.md must state 15 per-opponent pointer arrays"
    );
    let timer_doc =
        std::fs::read_to_string(manifest_dir.join("../../docs/reference/round-timer.md"))
            .expect("read round-timer.md");
    assert!(
        !timer_doc.contains("not set in any capture") && timer_doc.contains("sf-select-usa-01"),
        "round-timer.md must record 0x10 verified in country-select captures"
    );
    let fixed_json = std::fs::read_to_string(manifest_dir.join(FIXED_FIGHT_SCENARIO))
        .expect("read sf_fixed_fight_01.json");
    assert!(
        fixed_json.contains("country select already exited on timeout at 1865")
            && !fixed_json.contains("confirming the default country"),
        "sf_fixed_fight_01.json must note that country select ends by timeout at 1865"
    );

    // Criterion 6 (C-NEG): LP 10 frames late (1658 instead of 1648 in
    // sf-select-usa-01, so release is at 1671 instead of 1661) is caught at
    // frame 1661 by both compare_cpu_ram_with_inputs and sf_compare::compare.
    let orig_usa = std::fs::read_to_string(&usa_scenario).unwrap();
    let delayed_usa = orig_usa.replace(r#""frame": 1648"#, r#""frame": 1658"#);
    assert_ne!(orig_usa, delayed_usa, "LP at 1648 replaced with 1658");

    let scratch = ScratchDir::new("sf-select-usa-01-neg");
    std::fs::create_dir_all(&*scratch).unwrap();
    let delayed_path = scratch.join("delayed_usa.json");
    std::fs::write(&delayed_path, delayed_usa).unwrap();

    let ram_outcome = compare_cpu_ram_with_inputs(
        &usa_a,
        assets.maincpu.clone(),
        Some(&delayed_path),
        1500,
        1665,
    )
    .unwrap();
    match ram_outcome {
        CpuRamOutcome::Mismatch(msg) => {
            assert!(
                msg.contains("frame 1649"),
                "expected mismatch at frame 1649 when LP is delayed 10 frames, got: {msg}"
            );
        }
        other => panic!("expected CpuRamOutcome::Mismatch at frame 1649, got: {other:?}"),
    }

    let out_cap = scratch.join("capture");
    run_cpu_fight_capture_with_inputs(
        &usa_a,
        &assets,
        &out_cap,
        "sf-select-usa-01",
        Some(&delayed_path),
        1500,
        1665,
    )
    .unwrap();
    let delayed_scen = load_scenario(&delayed_path).unwrap();
    let ref_cap = load_capture(&usa_a).unwrap();
    let cand_cap = load_capture(&out_cap).unwrap();
    let scope = CompareScope::default().with_range(1501, 1665).with_fields(
        CompareScope::parse_fields("screen_hash,latch,latch_changed")
            .expect("valid observable field names"),
    );
    let cmp_outcome = compare(
        &delayed_scen,
        &ref_cap,
        &cand_cap,
        CandidateKind::Native,
        &scope,
    )
    .unwrap();
    match cmp_outcome {
        Outcome::Mismatch(m) => {
            assert_eq!(
                m.frame, 1661,
                "capture compare catches delayed LP at frame 1661"
            );
        }
        other => panic!("expected Outcome::Mismatch at frame 1661, got: {other:?}"),
    }
}

/// Ticket #86 (`F46`): a fresh two-player start into the versus fight:
/// 1. Uses `sf-start-2p-01` (`sf-move-01`'s boot inputs, checkpoints `1440..=2450`);
///    no new capture is needed.
/// 2. `C-NAT` from its `1500` checkpoint (and `1440`) over `1501..=2450`: the 2P
///    start at `1561`, the select in mode 1 (`1561..=1905`), events 2 and 3
///    (`1906..=1997`), the mode-1 placement at `0x3e5ea` (`1998`), event 4's
///    versus shape (`1998..=2149`), `ROUND 1` (`2150..=2318`), walk-in intro
///    (`2319..=2399`), `FIGHT!` (`2400..=2429`), and round start at `2430`.
/// 3. At `2450` the owned RAM equals `sf-move-01`'s `2450` fixture (`move-run-a`),
///    and the run continues on `F26` (#66)'s shared `FightMachine` with
///    `sf-move-01`'s inputs over `2451..=3350` against `sf-move-01`'s checkpoints.
/// 4. `C-NEG`: P1's and P2's placement words swapped are caught at the first
///    frame the fighters are drawn (`2319`).
#[test]
fn two_player_start_into_versus_fight_matches_1500_to_2450_and_continues_through_3350() {
    let Some(Captures {
        assets,
        s2_a,
        move_a,
        ..
    }) = require_captures()
    else {
        eprintln!("skipping: start-2p / move reference captures not present");
        return;
    };
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let s2_scenario = manifest_dir.join(START_2P_SCENARIO);
    let move_scenario = manifest_dir.join("../../tools/reference/scenarios/sf_move_01.json");

    // Criterion 2 (C-NAT): verify 1500..=2450 (and 1440..=2450) across all
    // observables (Scheduler::from_fixture, full work RAM, CPU_RAM_REGIONS,
    // screen_hash, latch, latch_changed, checkpoint pixels and RAM).
    verify_native_interval(&assets, &s2_a, &s2_scenario, "sf-start-2p-01", 1500, 2450);
    verify_native_interval(&assets, &s2_a, &s2_scenario, "sf-start-2p-01", 1440, 2450);

    // Verify the mode-1 pre-fight milestones along 1500..=2450 and capture
    // the native work RAM at frame 2450.
    let mut placement_1998 = None;
    let mut work_2450 = None;
    let mut milestones = Vec::new();
    let (stopped, unrec) = observe_cpu_fight(
        &s2_a,
        assets.maincpu.clone(),
        Some(&s2_scenario),
        1500,
        2450,
        |m| {
            let f = m.state.frame;
            let w = cpu_work_image(m);
            if matches!(f, 1561 | 1906 | 1998 | 2150 | 2319 | 2400 | 2430 | 2450) {
                milestones.push((f, w[0x4002], w[0x0e36], w[0x4008], w[0x6409], w[0x6a09]));
            }
            if f == 1998 {
                placement_1998 = Some((
                    w[0x06ba..0x06c2].to_vec(),
                    u16::from_be_bytes([w[0x06d0], w[0x06d1]]),
                    w[0x06e4..0x06ec].to_vec(),
                    u16::from_be_bytes([w[0x06fa], w[0x06fb]]),
                    u32::from_be_bytes([w[0x6438], w[0x6439], w[0x643a], w[0x643b]]),
                    u32::from_be_bytes([w[0x643c], w[0x643d], w[0x643e], w[0x643f]]),
                    u32::from_be_bytes([w[0x6a38], w[0x6a39], w[0x6a3a], w[0x6a3b]]),
                    u32::from_be_bytes([w[0x6a3c], w[0x6a3d], w[0x6a3e], w[0x6a3f]]),
                ));
            }
            if f == 2450 {
                work_2450 = Some(cpu_work_image(m));
            }
        },
    )
    .unwrap();
    assert_eq!((stopped, unrec), (2450, Vec::new()));
    assert_eq!(
        milestones,
        vec![
            (1561, 1, 0, 0x00, 0x00, 0x00), // 2P start -> GAME_MODE=1, country select
            (1906, 1, 2, 0x20, 0x00, 0x00), // select timeout -> StageStart requests event 2
            (1998, 1, 4, 0x20, 0x00, 0x00), // events 2 & 3 done -> place_players(mode 1), event 4
            (2150, 1, 5, 0x20, 0x00, 0x00), // VersusIntro done -> event 5 (ROUND 1)
            (2319, 1, 5, 0x00, 0x80, 0x80), // round_hud -> fighters placed & walking in
            (2400, 1, 5, 0x00, 0x00, 0x00), // walk-in done -> FIGHT! shown
            (2430, 1, 5, 0x80, 0x12, 0x12), // RoundStart -> fight begins
            (2450, 1, 5, 0x80, 0x12, 0x12), // sf-move-01 fixture frame 2450
        ]
    );
    let (p1_words, p1_score, p2_words, p2_score, e1_own, e1_opp, e2_own, e2_opp) =
        placement_1998.expect("placement at 1998");
    assert_eq!(
        p1_words,
        vec![0x02, 0xbc, 0x02, 0x98, 0x0a, 0x00, 0x04, 0x00]
    );
    assert_eq!(p1_score, 0x019a);
    assert_eq!(
        p2_words,
        vec![0x03, 0xbc, 0x02, 0xe0, 0x04, 0x00, 0x0a, 0x00]
    );
    assert_eq!(p2_score, 0x01dc);
    assert_eq!(
        (e1_own, e1_opp, e2_own, e2_opp),
        (0xff86b0, 0xff86da, 0xff86da, 0xff86b0)
    );

    // Criterion 3: at 2450 the owned RAM equals sf-move-01's 2450 fixture,
    // and the run continues on F26 (#66)'s machine with sf-move-01's inputs
    // over 2451..=3350 against sf-move-01's checkpoints.
    let work_2450 = work_2450.expect("native work RAM at 2450");
    let move_2450 = std::fs::read(move_a.join("ram_002450_work.bin")).unwrap();
    let owned = sf_engine::run::fight_owned_addresses();
    for &(name, addr) in &owned {
        let idx = (addr - 0xff8000) as usize;
        assert_eq!(
            work_2450[idx], move_2450[idx],
            "at frame 2450 owned RAM {name} at 0x{addr:06x} must equal sf-move-01's 2450 fixture"
        );
    }

    // Continue from 1500 in sf-start-2p-01 all the way through 3350 using
    // sf-move-01's inputs (which include the exact same boot inputs at
    // 1450/1480/1560 plus the versus movement inputs at 2460..=3350),
    // comparing owned RAM at every frame 2450..=3350 against move-run-a.
    let mut checked_move_frames = 0u32;
    let (stopped_3350, unrec_3350) = observe_cpu_fight(
        &s2_a,
        assets.maincpu.clone(),
        Some(&move_scenario),
        1500,
        3350,
        |m| {
            let f = m.state.frame;
            if f < 2450 {
                return;
            }
            let ref_w = std::fs::read(move_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
            let nat_w = cpu_work_image(m);
            for &(name, addr) in &owned {
                let idx = (addr - 0xff8000) as usize;
                assert_eq!(
                    nat_w[idx], ref_w[idx],
                    "frame {f}: {name} at 0x{addr:06x} differs from move-run-a"
                );
            }
            checked_move_frames += 1;
        },
    )
    .unwrap();
    assert_eq!((stopped_3350, unrec_3350), (3350, Vec::new()));
    assert_eq!(checked_move_frames, 3350 - 2450 + 1);

    // Also verify F26 (#66)'s compare_fight_ram_with_inputs and run_fight_capture
    // + sf_compare::compare over 2451..=3350 against move-run-a.
    let fight_ram = sf_engine::run::compare_fight_ram_with_inputs(
        &move_a,
        assets.maincpu.clone(),
        Some(&move_scenario),
        2450,
        3350,
    )
    .unwrap();
    assert_eq!(
        fight_ram,
        CpuRamOutcome::Match {
            frames: 3350 - 2450
        }
    );

    let move_out = ScratchDir::new("sf-start-2p-move-continue");
    let move_frames = sf_engine::run::run_fight_capture(
        &move_a,
        &assets,
        &move_out,
        "sf-move-01",
        Some(&move_scenario),
        2450,
        3350,
    )
    .unwrap();
    assert_eq!(move_frames, 3350 - 2450 + 1);
    let move_scen = load_scenario(&move_scenario).unwrap();
    let move_ref = load_capture(&move_a).unwrap();
    let move_cand = load_capture(&move_out).unwrap();
    let move_scope = CompareScope::default().with_range(2451, 3350).with_fields(
        CompareScope::parse_fields("screen_hash,latch,latch_changed")
            .expect("valid observable field names"),
    );
    let move_cmp = compare(
        &move_scen,
        &move_ref,
        &move_cand,
        CandidateKind::Native,
        &move_scope,
    )
    .unwrap();
    assert!(
        matches!(
            move_cmp,
            Outcome::Match {
                frames_compared: 900,
                checkpoints_compared: 900
            }
        ),
        "expected Match over 2451..=3350, got: {move_cmp:?}"
    );

    // Criterion 4 (C-NEG):
    // (a) Swapping P1's and P2's placement words (+0xa, +0xc, +0xe, +0x10,
    //     +0x20) at frame 2318 before round_hud (0x3e714) draws the fighters
    //     into object RAM is caught at frame 2319 on object RAM (0xffe400).
    let swap_scratch = ScratchDir::new("sf-start-2p-swap-placement-neg");
    std::fs::create_dir_all(&*swap_scratch).unwrap();
    std::fs::copy(s2_a.join("frames.tsv"), swap_scratch.join("frames.tsv")).unwrap();
    for f in [2318u32, 2319, 2320] {
        for kind in ["work", "video", "palette", "music"] {
            let name = format!("ram_{f:06}_{kind}.bin");
            std::fs::copy(s2_a.join(&name), swap_scratch.join(&name)).unwrap();
        }
    }
    let mut work_2318 = std::fs::read(swap_scratch.join("ram_002318_work.bin")).unwrap();
    for off in [0x0au32, 0x0c, 0x0e, 0x10, 0x20] {
        let i1 = (0x06b0 + off) as usize;
        let i2 = (0x06da + off) as usize;
        work_2318.swap(i1, i2);
        work_2318.swap(i1 + 1, i2 + 1);
    }
    std::fs::write(swap_scratch.join("ram_002318_work.bin"), &work_2318).unwrap();
    let swap_ram = compare_cpu_ram_with_inputs(
        &swap_scratch,
        assets.maincpu.clone(),
        Some(&s2_scenario),
        2318,
        2320,
    )
    .unwrap();
    match swap_ram {
        CpuRamOutcome::Mismatch(msg) => {
            assert!(
                msg.contains("frame 2319") && msg.contains("0xffe400"),
                "expected frame 2319 object RAM mismatch when placement words are swapped, got: {msg}"
            );
        }
        other => panic!("expected CpuRamOutcome::Mismatch at frame 2319, got: {other:?}"),
    }

    // (b) Swapping P1's and P2's initial X placement words (0x016062 and
    //     0x016632 in move 0's header) over a run from 1998 through 2320 is
    //     caught at frame 2319 (the first frame the fighters are drawn) by
    //     both compare_cpu_ram_with_inputs and sf_compare::compare.
    let mut bad_assets = assets;
    let p1_x = [bad_assets.maincpu[0x16062], bad_assets.maincpu[0x16063]];
    let p2_x = [bad_assets.maincpu[0x16632], bad_assets.maincpu[0x16633]];
    assert_ne!(
        p1_x, p2_x,
        "P1 and P2 initial X placement words must differ"
    );
    bad_assets.maincpu[0x16062..0x16064].copy_from_slice(&p2_x);
    bad_assets.maincpu[0x16632..0x16634].copy_from_slice(&p1_x);

    let swap_x_ram = compare_cpu_ram_with_inputs(
        &s2_a,
        bad_assets.maincpu.clone(),
        Some(&s2_scenario),
        1998,
        2320,
    )
    .unwrap();
    match swap_x_ram {
        CpuRamOutcome::Mismatch(msg) => {
            assert!(
                msg.contains("frame 2319"),
                "expected frame 2319 mismatch when P1/P2 X placement words are swapped, got: {msg}"
            );
        }
        other => panic!("expected CpuRamOutcome::Mismatch at frame 2319, got: {other:?}"),
    }

    let swap_out = ScratchDir::new("sf-start-2p-swap-placement-cand");
    run_cpu_fight_capture_with_inputs(
        &s2_a,
        &bad_assets,
        &swap_out,
        "sf-start-2p-01",
        Some(&s2_scenario),
        1998,
        2320,
    )
    .unwrap();
    let s2_scen = load_scenario(&s2_scenario).unwrap();
    let s2_ref = load_capture(&s2_a).unwrap();
    let swap_cand = load_capture(&swap_out).unwrap();
    let swap_scope = CompareScope::default().with_range(1999, 2320).with_fields(
        CompareScope::parse_fields("screen_hash,latch,latch_changed")
            .expect("valid observable field names"),
    );
    let swap_cmp = compare(
        &s2_scen,
        &s2_ref,
        &swap_cand,
        CandidateKind::Native,
        &swap_scope,
    )
    .unwrap();
    match swap_cmp {
        Outcome::Mismatch(m) => {
            assert_eq!(
                m.frame, 2319,
                "swapped P1/P2 placement words must be caught at frame 2319 (first frame fighters are drawn)"
            );
        }
        other => panic!("expected Outcome::Mismatch at frame 2319, got: {other:?}"),
    }
}

/// Ticket #87 (`F47`): four-country select (`0x3e1e4..0x3e371`, DIP
/// `Number of Countries Selected = 4`, `0xffc005 = 0`):
/// 1. `C-REF`: `sf-select4-01` and `sf-select4-02`, with `0xffc005 = 0`,
///    repeat byte-for-byte (`run-a == run-b`) with full checkpoints on every
///    frame from `1440` through event 2's request (`1761` / `1611`, and through
///    `1770` / `1620`).
/// 2. `C-NAT`: from each start-screen checkpoint (`1500` and `1440`) through
///    the button exit and event 2's request (`1762` in `sf-select4-01`, `1612`
///    in `sf-select4-02`): every cursor move across all four directions, the
///    ignored diagonals, and the marker redraws through `0x3e316`.
/// 3. The 15-entry pointer arrays for countries `1` (`0x19930 + 0xa0 * n`) and
///    `3` (`0x19980 + 0xa0 * n`) at `0xffc01c..0xffc057` equal the reference's.
/// 4. `C-NEG`: perturbing the 4-byte cursor-to-country map at `0x3e2ea` is
///    caught at the first cursor move (`1541`) by both
///    `compare_cpu_ram_with_inputs` and `sf_compare::compare`.
#[test]
fn four_country_select_dip_cursor_moves_ignored_diagonals_and_country_1_and_3_arrays_match_reference(
) {
    let Some(roms) = local(ROMS) else {
        eprintln!("skipping: ROMs not present");
        return;
    };
    let (Some(s4_01_a), Some(s4_01_b), Some(s4_02_a), Some(s4_02_b)) = (
        local("../../tools/reference/local/captures/select4-01-run-a"),
        local("../../tools/reference/local/captures/select4-01-run-b"),
        local("../../tools/reference/local/captures/select4-02-run-a"),
        local("../../tools/reference/local/captures/select4-02-run-b"),
    ) else {
        eprintln!("skipping: select4-01 / select4-02 reference captures not present");
        return;
    };
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = manifest_dir.join(MANIFEST);
    let assets = load_assets(&roms, &manifest).expect("ROM set assembles");
    let s4_01_scenario = manifest_dir.join("../../tools/reference/scenarios/sf_select4_01.json");
    let s4_02_scenario = manifest_dir.join("../../tools/reference/scenarios/sf_select4_02.json");

    // 1. C-REF: run-a == run-b and 0xffc005 == 0 across all checkpoints.
    assert_dir_identical(&s4_01_a, &s4_01_b, "sf-select4-01");
    assert_dir_identical(&s4_02_a, &s4_02_b, "sf-select4-02");
    for f in 1440..=1770u32 {
        let w = std::fs::read(s4_01_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
        assert_eq!(w[0x4005], 0, "sf-select4-01 frame {f}: 0xffc005 must be 0");
    }
    for f in 1440..=1620u32 {
        let w = std::fs::read(s4_02_a.join(format!("ram_{f:06}_work.bin"))).unwrap();
        assert_eq!(w[0x4005], 0, "sf-select4-02 frame {f}: 0xffc005 must be 0");
    }

    // 2. C-NAT from 1500 and from 1440 through button exit, event 2's request,
    // and 0x3f latch (1762 in sf-select4-01, 1612 in sf-select4-02).
    verify_native_interval(
        &assets,
        &s4_01_a,
        &s4_01_scenario,
        "sf-select4-01",
        1500,
        1762,
    );
    verify_native_interval(
        &assets,
        &s4_01_a,
        &s4_01_scenario,
        "sf-select4-01",
        1440,
        1762,
    );
    verify_native_interval(
        &assets,
        &s4_02_a,
        &s4_02_scenario,
        "sf-select4-02",
        1500,
        1612,
    );
    verify_native_interval(
        &assets,
        &s4_02_a,
        &s4_02_scenario,
        "sf-select4-02",
        1440,
        1612,
    );

    // Verify every cursor move, ignored diagonal, marker redraw, and country 1
    // pointer array in sf-select4-01.
    let mut country_trace_01 = Vec::new();
    let mut vram_at_1691 = Vec::new();
    let mut array_01 = Vec::new();
    let (stopped_01, unrec_01) = observe_cpu_fight(
        &s4_01_a,
        assets.maincpu.clone(),
        Some(&s4_01_scenario),
        1500,
        1762,
        |m| {
            let f = m.state.frame;
            let w = m.work_ram();
            if f == 1521 {
                assert_eq!(m.state.fg_scroll, 0x4380, "frame 1521: fg_scroll == 0x4380");
                assert_eq!(
                    u32::from_be_bytes([w[0x0634], w[0x0635], w[0x0636], w[0x0637]]),
                    0x1a668,
                    "frame 1521: palette 0x1a668 requested"
                );
                assert_eq!(w[0x0682], 0x48, "frame 1521: gfxctrl == 0x48");
            }
            if matches!(
                f,
                1521 | 1523 | 1541 | 1571 | 1601 | 1631 | 1661 | 1691 | 1725 | 1761 | 1762
            ) {
                country_trace_01.push((f, w[0x4006], w[0x4008], w[0x0e36]));
            }
            if f == 1691 {
                vram_at_1691 = m.state.videoram.clone();
            }
            if (1721..=1732).contains(&f) {
                assert_eq!(
                    w[0x4006], 1,
                    "frame {f}: up-left diagonal must be ignored (country stays 1)"
                );
                assert_eq!(
                    m.state.videoram, vram_at_1691,
                    "frame {f}: marker vram unchanged during ignored diagonal"
                );
            }
            if f == 1761 {
                for n in 0..15usize {
                    let off = 0x401c + 4 * n;
                    let ptr = u32::from_be_bytes([w[off], w[off + 1], w[off + 2], w[off + 3]]);
                    array_01.push(ptr);
                }
            }
            if f == 1762 {
                assert_eq!(m.state.sound_latch, 0x3f, "frame 1762: sound_latch == 0x3f");
            }
        },
    )
    .unwrap();
    assert_eq!((stopped_01, unrec_01), (1762, Vec::new()));
    assert_eq!(
        country_trace_01,
        vec![
            (1521, 0, 0x00, 0), // 0x3e1e4: initial Japan (0)
            (1523, 0, 0x02, 0), // 0x3e27c: first poll pass (D6=0, D7=0 -> Japan 0)
            (1541, 2, 0x02, 0), // Right -> D6=1, D7=0 -> USA (2)
            (1571, 1, 0x02, 0), // Down  -> D6=1, D7=1 -> China (1)
            (1601, 3, 0x02, 0), // Left  -> D6=0, D7=1 -> England (3)
            (1631, 0, 0x02, 0), // Up    -> D6=0, D7=0 -> Japan (0)
            (1661, 3, 0x02, 0), // Down  -> D6=0, D7=1 -> England (3)
            (1691, 1, 0x02, 0), // Right -> D6=1, D7=1 -> China (1)
            (1725, 1, 0x02, 0), // Up+Left diagonal ignored -> stays China (1)
            (1761, 1, 0x00, 0), // LP exit -> clears SCENE_MODE, fills array, sleeps into StageStart
            (1762, 1, 0x20, 2), // StageStart wakes up and requests event 2
        ]
    );

    // Criterion 3 (country 1 China): pointer array equals 0x19930 + 0xa0 * n
    // and matches the reference capture's 0xffc01c..0xffc057.
    let ref_1761 = std::fs::read(s4_01_a.join("ram_001761_work.bin")).unwrap();
    assert_eq!(array_01.len(), 15);
    for (n, &ptr) in array_01.iter().enumerate() {
        let off = 0x401c + 4 * n;
        let ref_ptr = u32::from_be_bytes([
            ref_1761[off],
            ref_1761[off + 1],
            ref_1761[off + 2],
            ref_1761[off + 3],
        ]);
        assert_eq!(ptr, ref_ptr, "China array[{n}] matches reference");
        assert_eq!(
            ptr,
            0x19930 + 0xa0 * n as u32,
            "China array[{n}] == 0x19930 + 0xa0 * {n}"
        );
    }

    // Verify sf-select4-02 (Down -> England 3, ignored Up+Right diagonal, LP
    // exit -> country 3 pointer array at 0x19980 + 0xa0 * n).
    let mut country_trace_02 = Vec::new();
    let mut array_02 = Vec::new();
    let (stopped_02, unrec_02) = observe_cpu_fight(
        &s4_02_a,
        assets.maincpu.clone(),
        Some(&s4_02_scenario),
        1500,
        1612,
        |m| {
            let f = m.state.frame;
            let w = m.work_ram();
            if matches!(f, 1521 | 1541 | 1575 | 1611 | 1612) {
                country_trace_02.push((f, w[0x4006], w[0x4008], w[0x0e36]));
            }
            if f == 1611 {
                for n in 0..15usize {
                    let off = 0x401c + 4 * n;
                    let ptr = u32::from_be_bytes([w[off], w[off + 1], w[off + 2], w[off + 3]]);
                    array_02.push(ptr);
                }
            }
            if f == 1612 {
                assert_eq!(m.state.sound_latch, 0x3f, "frame 1612: sound_latch == 0x3f");
            }
        },
    )
    .unwrap();
    assert_eq!((stopped_02, unrec_02), (1612, Vec::new()));
    assert_eq!(
        country_trace_02,
        vec![
            (1521, 0, 0x00, 0), // 0x3e1e4: initial Japan (0)
            (1541, 3, 0x02, 0), // Down -> D6=0, D7=1 -> England (3)
            (1575, 3, 0x02, 0), // Up+Right diagonal ignored -> stays England (3)
            (1611, 3, 0x00, 0), // LP exit -> clears SCENE_MODE, fills array, sleeps into StageStart
            (1612, 3, 0x20, 2), // StageStart wakes up and requests event 2
        ]
    );

    // Criterion 3 (country 3 England): pointer array equals 0x19980 + 0xa0 * n
    // and matches the reference capture's 0xffc01c..0xffc057.
    let ref_1611 = std::fs::read(s4_02_a.join("ram_001611_work.bin")).unwrap();
    assert_eq!(array_02.len(), 15);
    for (n, &ptr) in array_02.iter().enumerate() {
        let off = 0x401c + 4 * n;
        let ref_ptr = u32::from_be_bytes([
            ref_1611[off],
            ref_1611[off + 1],
            ref_1611[off + 2],
            ref_1611[off + 3],
        ]);
        assert_eq!(ptr, ref_ptr, "England array[{n}] matches reference");
        assert_eq!(
            ptr,
            0x19980 + 0xa0 * n as u32,
            "England array[{n}] == 0x19980 + 0xa0 * {n}"
        );
    }

    // 4. C-NEG: perturbing 0x3e2eb (the top-right entry of the 4-byte map at
    // 0x3e2ea, from 2 USA to 1 China) is caught at frame 1541 (the first
    // cursor move) by both compare_cpu_ram_with_inputs and sf_compare::compare.
    let mut bad_assets = assets;
    assert_eq!(&bad_assets.maincpu[0x3e2ea..0x3e2ee], &[0, 2, 3, 1]);
    bad_assets.maincpu[0x3e2eb] = 1;

    let neg_ram = compare_cpu_ram_with_inputs(
        &s4_01_a,
        bad_assets.maincpu.clone(),
        Some(&s4_01_scenario),
        1500,
        1545,
    )
    .unwrap();
    match neg_ram {
        CpuRamOutcome::Mismatch(msg) => {
            assert!(
                msg.contains("frame 1541") && msg.contains("0xffc006"),
                "expected frame 1541 mismatch on 0xffc006, got: {msg}"
            );
        }
        other => panic!("expected CpuRamOutcome::Mismatch at frame 1541, got: {other:?}"),
    }

    let neg_out = ScratchDir::new("sf-select4-01-neg-cand");
    run_cpu_fight_capture_with_inputs(
        &s4_01_a,
        &bad_assets,
        &neg_out,
        "sf-select4-01",
        Some(&s4_01_scenario),
        1500,
        1545,
    )
    .unwrap();
    let s4_scen = load_scenario(&s4_01_scenario).unwrap();
    let s4_ref = load_capture(&s4_01_a).unwrap();
    let neg_cand = load_capture(&neg_out).unwrap();
    let neg_scope = CompareScope::default().with_range(1501, 1545).with_fields(
        CompareScope::parse_fields("screen_hash,latch,latch_changed")
            .expect("valid observable field names"),
    );
    let neg_cmp = compare(
        &s4_scen,
        &s4_ref,
        &neg_cand,
        CandidateKind::Native,
        &neg_scope,
    )
    .unwrap();
    match neg_cmp {
        Outcome::Mismatch(m) => {
            assert_eq!(
                m.frame, 1541,
                "perturbed 0x3e2ea map must be caught at frame 1541"
            );
        }
        other => panic!("expected Outcome::Mismatch at frame 1541, got: {other:?}"),
    }
}
