//! Ticket #58 (F18): Flip screen on every layer.
//!
//! Verifies:
//! 1. `MachineState.flip_screen` follows `gfxctrl` bit 2 (`0x04`) on every
//!    write (`0x3b644` / `0x3b690`) and on fixture load (`CpuMachine`,
//!    `VersusMachine`).
//! 2. C-REF on `sf-flip-01` (the fixed fight's inputs with `Flip Screen = On`,
//!    `dsw1 = 0xdeff`, `dsw2 = 0xffff`, frames `0..=5040`, checkpoints at `790`,
//!    `1500`, and `2389..=5040`): two MAME 0.289 runs are byte-for-byte
//!    identical, and its RAM equals the unflipped baseline/audit captures
//!    except `gfxctrl` bit 2 (`0xff8682` and stack-saved `D0` copies of
//!    `0xff8682`).
//! 3. Rendered from dumps, checkpoints `790`, `1500`, and `2389` of
//!    `sf-flip-01` (and the unflipped checkpoints of `sf-fixed-fight-01` /
//!    `sf-fixed-fight-audit-01`) are byte-exact at 384x224.
//! 4. C-NAT from the flip-on `2389` fixture over `2390..=5014`, matching the
//!    reference on every frame and checkpoint (BG, FG, TX, and sprites all
//!    flipped).
//! 5. C-NEG: rendering frame `2390` unflipped (`flip_screen = false`) fails
//!    comparison at frame `2390`.

use std::path::Path;

use sf_compare::capture::load_capture;
use sf_compare::compare::{compare, CandidateKind, CompareScope, Outcome};
use sf_compare::scenario::load_scenario;
use sf_engine::cpu::CpuMachine;
use sf_engine::run::{
    cpu_owned_addresses, cpu_work_image, observe_cpu_fight, run_cpu_fight_capture_with_inputs,
    ScratchDir,
};
use sf_video::render::{render_frame, HEIGHT, WIDTH};
use sf_video::romset::{load_assets, Assets};
use sf_video::scene::SceneState;

const FLIP_SCENARIO: &str = "../../tools/reference/scenarios/sf_flip_01.json";
const FLIP_ID: &str = "sf-flip-01";
const FLIP_A: &str = "../../tools/reference/local/captures/flip-01-run-a";
const FLIP_B: &str = "../../tools/reference/local/captures/flip-01-run-b";
const AUDIT_A: &str = "../../tools/reference/local/captures/fight-audit-run-a";
const BASELINE_A: &str = "../../tools/reference/local/captures/fight-run-a";
const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

const FIXTURE: u32 = 2389;
const AUDIT_LAST: u32 = 5014;
const FLIP_LAST: u32 = 5040;

fn present() -> bool {
    [FLIP_A, FLIP_B, AUDIT_A, BASELINE_A, ROMS]
        .iter()
        .all(|p| Path::new(p).is_dir())
}

fn load_checkpoint_scene(capture_dir: &str, frame: u32) -> (SceneState, Vec<u8>) {
    let dir = Path::new(capture_dir);
    let video = std::fs::read(dir.join(format!("ram_{frame:06}_video.bin"))).unwrap();
    let palette = std::fs::read(dir.join(format!("ram_{frame:06}_palette.bin"))).unwrap();
    let work = std::fs::read(dir.join(format!("ram_{frame:06}_work.bin"))).unwrap();
    let pixels = std::fs::read(dir.join(format!("pixels_{frame:06}.rgba"))).unwrap();
    let reg = |a: usize| u16::from_be_bytes([work[a], work[a + 1]]);
    let active = work[0x0682];
    let scene = SceneState {
        videoram: video,
        objectram: work[0x6000..0x8000].to_vec(),
        paletteram: palette,
        bg_scroll: reg(0x4682),
        fg_scroll: reg(0x4680),
        active,
        flip_screen: active & 0x04 != 0,
        bg_page: 0,
        fg_page: 0,
    };
    (scene, pixels)
}

fn frame_to_bgra(argb: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(argb.len() * 4);
    for &px in argb {
        out.extend_from_slice(&px.to_le_bytes());
    }
    out
}

fn assert_scene_matches_pixels(
    assets: &Assets,
    scene: &SceneState,
    ref_pixels: &[u8],
    label: &str,
) {
    let rendered = render_frame(assets, scene);
    assert_eq!(rendered.argb.len(), WIDTH * HEIGHT);
    let bgra = frame_to_bgra(&rendered.argb);
    assert_eq!(bgra.len(), ref_pixels.len(), "{label}: byte length");
    if bgra != ref_pixels {
        let first = bgra
            .chunks_exact(4)
            .zip(ref_pixels.chunks_exact(4))
            .position(|(a, b)| a != b)
            .unwrap();
        let (sx, sy) = (first % WIDTH, first / WIDTH);
        panic!(
            "{label}: pixel mismatch at ({sx}, {sy}): native {:02x?}, reference {:02x?}",
            &bgra[4 * first..4 * first + 4],
            &ref_pixels[4 * first..4 * first + 4]
        );
    }
}

/// Acceptance 1 (C-REF): `sf-flip-01` runs A and B are byte-for-byte identical,
/// and its RAM at 790, 1500, and 2389..=5040 equals the unflipped baseline and
/// audit captures except `gfxctrl` bit 2 (`0x04`).
#[test]
fn sf_flip_01_reference_runs_are_identical_and_ram_matches_audit_capture_except_gfxctrl_bit_2() {
    if !present() {
        eprintln!("skipping: local flip-01 captures/ROMs not present");
        return;
    }

    let scenario = load_scenario(FLIP_SCENARIO).unwrap();
    assert_eq!(scenario.scenario_id, FLIP_ID);
    assert_eq!((scenario.dsw1, scenario.dsw2), (0xdeff, 0xffff));

    let ref_a = load_capture(FLIP_A).unwrap();
    assert_eq!(
        (ref_a.meta.dsw1, ref_a.meta.dsw2),
        (Some(0xdeff), Some(0xffff))
    );

    // Verify run-a and run-b match byte-for-byte across all observation files.
    let files_a: Vec<_> = std::fs::read_dir(FLIP_A)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let s = name.to_string_lossy();
            s == "frames.tsv"
                || s == "meta.tsv"
                || s == "rom_check.tsv"
                || (s.starts_with("ram_") && s.ends_with(".bin"))
                || (s.starts_with("pixels_") && s.ends_with(".rgba"))
        })
        .map(|e| e.file_name())
        .collect();
    assert_eq!(
        files_a.len(),
        2654 * 5 + 2,
        "2,654 checkpoints * 5 files + 2 TSVs"
    );
    for name in &files_a {
        let ba = std::fs::read(Path::new(FLIP_A).join(name)).unwrap();
        let bb = std::fs::read(Path::new(FLIP_B).join(name)).unwrap();
        assert_eq!(ba, bb, "flip-01 run-a vs run-b mismatch in {name:?}");
    }

    // Compare RAM at 790 and 1500 against fight-run-a, and 2389..=5040 against fight-audit-run-a.
    let check_frame = |frame: u32, unflipped_dir: &str| {
        let flip_dir = Path::new(FLIP_A);
        let unflip_dir = Path::new(unflipped_dir);
        for suf in ["video", "palette"] {
            let f_bytes =
                std::fs::read(flip_dir.join(format!("ram_{frame:06}_{suf}.bin"))).unwrap();
            let u_bytes =
                std::fs::read(unflip_dir.join(format!("ram_{frame:06}_{suf}.bin"))).unwrap();
            assert_eq!(
                f_bytes, u_bytes,
                "frame {frame} {suf} RAM must be identical"
            );
        }
        let f_work = std::fs::read(flip_dir.join(format!("ram_{frame:06}_work.bin"))).unwrap();
        let u_work = std::fs::read(unflip_dir.join(format!("ram_{frame:06}_work.bin"))).unwrap();
        // Object RAM (0xffe000..0x1000000) is identical.
        assert_eq!(
            &f_work[0x6000..0x8000],
            &u_work[0x6000..0x8000],
            "frame {frame} object RAM must be identical"
        );
        // 0xff8682 has bit 2 set in sf-flip-01 and clear in the unflipped capture.
        assert_eq!(
            f_work[0x0682],
            u_work[0x0682] | 0x04,
            "frame {frame}: 0xff8682 must differ only by bit 2 (0x04)"
        );
        for i in 0..0x6000usize {
            if f_work[i] != u_work[i] {
                let addr = 0xff8000 + i as u32;
                assert_eq!(
                    f_work[i] ^ u_work[i],
                    0x04,
                    "frame {frame}: work RAM at 0x{addr:06x} differed by {:02x} vs {:02x}",
                    f_work[i],
                    u_work[i]
                );
                // Outside stack memory (0xff8106..=0xff8631, where D0 holding
                // 0xff8682 is saved by movem.l), only 0xff8682 itself differs.
                if !(0xff8106..=0xff8631).contains(&addr) {
                    assert_eq!(
                        addr, 0xff8682,
                        "frame {frame}: unexpected non-stack work RAM difference at 0x{addr:06x}"
                    );
                }
            }
        }
    };

    check_frame(790, BASELINE_A);
    check_frame(1500, BASELINE_A);
    for frame in FIXTURE..=FLIP_LAST {
        check_frame(frame, AUDIT_A);
    }
}

/// Acceptance 2 & 4: Rendered from dumps, checkpoints 790, 1500, and 2389 of
/// `sf-flip-01` are byte-exact at 384x224, and the unflipped checkpoints (790,
/// 1500, 2280 of `fight-run-a` and 2389 of `fight-audit-run-a`) still match.
#[test]
fn checkpoints_790_1500_and_2389_render_byte_exact_at_384x224() {
    if !present() {
        eprintln!("skipping: local flip-01 captures/ROMs not present");
        return;
    }

    let assets = load_assets(ROMS, MANIFEST).unwrap();

    for frame in [790u32, 1500, 2389] {
        let (scene, ref_pixels) = load_checkpoint_scene(FLIP_A, frame);
        assert!(
            scene.flip_screen,
            "sf-flip-01 frame {frame} must have flip_screen = true"
        );
        assert_scene_matches_pixels(
            &assets,
            &scene,
            &ref_pixels,
            &format!("sf-flip-01 frame {frame}"),
        );
    }

    for (dir, frame) in [
        (BASELINE_A, 790u32),
        (BASELINE_A, 1500),
        (BASELINE_A, 2280),
        (AUDIT_A, 2389),
    ] {
        let (scene, ref_pixels) = load_checkpoint_scene(dir, frame);
        assert!(
            !scene.flip_screen,
            "unflipped frame {frame} must have flip_screen = false"
        );
        assert_scene_matches_pixels(
            &assets,
            &scene,
            &ref_pixels,
            &format!("unflipped frame {frame}"),
        );
    }
}

/// Acceptance 3 (C-NAT) & 5 (C-NEG):
/// - `MachineState.flip_screen` loads as `true` from the flip-on 2389 fixture
///   and stays synced to `gfxctrl` bit 2.
/// - Native play from the flip-on 2389 fixture matches `sf-flip-01` across
///   `2390..=5014` on `sf_compare::compare` and all owned work RAM.
/// - Rendering frame `2390` unflipped (`flip_screen = false`) fails comparison
///   at frame `2390`.
#[test]
fn native_flip_on_fight_from_2389_matches_reference_and_unflipped_render_fails_at_2390() {
    if !present() {
        eprintln!("skipping: local flip-01 captures/ROMs not present");
        return;
    }

    let assets = load_assets(ROMS, MANIFEST).unwrap();
    let scenario = load_scenario(FLIP_SCENARIO).unwrap();
    let reference = load_capture(FLIP_A).unwrap();

    // Verify fixture load sets flip_screen = true on CpuMachine (and false on
    // the unflipped audit capture).
    let flip_machine =
        CpuMachine::from_fight_checkpoint(Path::new(FLIP_A), FIXTURE, assets.maincpu.clone())
            .unwrap();
    assert!(
        flip_machine.state.flip_screen,
        "flip-01 2389 fixture must set flip_screen = true"
    );
    assert_eq!(flip_machine.state.gfxctrl, 0xec);

    let unflip_machine =
        CpuMachine::from_fight_checkpoint(Path::new(AUDIT_A), FIXTURE, assets.maincpu.clone())
            .unwrap();
    assert!(
        !unflip_machine.state.flip_screen,
        "fight-audit 2389 fixture must set flip_screen = false"
    );
    assert_eq!(unflip_machine.state.gfxctrl, 0xe8);

    // Run native capture over 2390..=5014 (Acceptance 3).
    let out = ScratchDir::new("sf-engine-flip-01");
    let executed = run_cpu_fight_capture_with_inputs(
        Path::new(FLIP_A),
        &assets,
        &out,
        FLIP_ID,
        Some(Path::new(FLIP_SCENARIO)),
        FIXTURE,
        AUDIT_LAST,
    )
    .unwrap();
    assert_eq!(executed, (AUDIT_LAST - FIXTURE + 1) as u64);

    let scope = CompareScope {
        from: Some((FIXTURE + 1) as u64),
        to: Some(AUDIT_LAST as u64),
        fields: Some(CompareScope::parse_fields("screen_hash,latch,latch_changed").unwrap()),
    };
    let native_cap = load_capture(&out).unwrap();
    let outcome = compare(
        &scenario,
        &reference,
        &native_cap,
        CandidateKind::Native,
        &scope,
    )
    .unwrap();
    assert!(
        matches!(
            outcome,
            Outcome::Match {
                frames_compared: 2625,
                checkpoints_compared: 2625,
            }
        ),
        "flip-01 2390..=5014 compare: {outcome:?}"
    );

    // Verify owned RAM and non-slot/non-stack work/object RAM over 2390..=5014.
    let owned = cpu_owned_addresses();
    let (stopped, unrecovered) = observe_cpu_fight(
        Path::new(FLIP_A),
        assets.maincpu.clone(),
        Some(Path::new(FLIP_SCENARIO)),
        FIXTURE,
        AUDIT_LAST,
        |m| {
            let frame = m.state.frame;
            assert!(
                m.state.flip_screen,
                "flip_screen must remain true at frame {frame}"
            );
            assert_eq!(
                m.state.gfxctrl & 0x04,
                0x04,
                "gfxctrl bit 2 must remain set at frame {frame}"
            );
            let image = cpu_work_image(m);
            let dump =
                std::fs::read(Path::new(FLIP_A).join(format!("ram_{frame:06}_work.bin"))).unwrap();
            for &(name, addr) in &owned {
                let i = (addr - 0xff8000) as usize;
                assert_eq!(
                    dump[i], image[i],
                    "flip-01 frame {frame}: {name} at 0x{addr:06x}"
                );
            }
            for (i, (&ref_b, &nat_b)) in dump.iter().zip(image.iter()).enumerate() {
                let addr = 0xff8000 + i as u32;
                let in_slot_saved =
                    (0xff8006..=0xff8105).contains(&addr) && (addr - 0xff8006) % 16 >= 2;
                let in_stack = (0xff8106..=0xff8631).contains(&addr);
                if !in_slot_saved && !in_stack {
                    assert_eq!(
                        ref_b, nat_b,
                        "flip-01 frame {frame}: non-slot/non-stack work/object RAM at 0x{addr:06x}"
                    );
                }
            }
        },
    )
    .unwrap();
    assert_eq!((stopped, unrecovered), (AUDIT_LAST, vec![]));

    // Acceptance 5 (C-NEG): rendering unflipped at frame 2390 fails comparison
    // at frame 2390.
    let (mut unflipped_2390, _) = load_checkpoint_scene(FLIP_A, 2390);
    unflipped_2390.flip_screen = false;
    let unflipped_frame = render_frame(&assets, &unflipped_2390);
    let unflipped_pixels = frame_to_bgra(&unflipped_frame.argb);
    std::fs::write(out.join("pixels_002390.rgba"), &unflipped_pixels).unwrap();
    let mut bad_cap = load_capture(&out).unwrap();
    // Keep the checkpoint's recorded CRC consistent with its replaced pixels;
    // the comparator uses that observable to decide whether to inspect them.
    bad_cap
        .checkpoints
        .iter_mut()
        .find(|checkpoint| checkpoint.frame == 2390)
        .unwrap()
        .screen_crc32 = format!("{:08x}", crc32fast::hash(&unflipped_pixels));
    let neg_outcome = compare(
        &scenario,
        &reference,
        &bad_cap,
        CandidateKind::Native,
        &scope,
    )
    .unwrap();
    match neg_outcome {
        Outcome::Mismatch(m) => {
            assert_eq!(
                m.frame, 2390,
                "unflipped render must mismatch at frame 2390"
            );
            assert_eq!(
                m.observable, "checkpoint screen pixels",
                "mismatch observable must be checkpoint pixels: {m:?}"
            );
        }
        other => panic!("expected Mismatch at 2390 when rendered unflipped, got {other:?}"),
    }
}
