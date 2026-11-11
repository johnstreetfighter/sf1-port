//! Ticket #169 (`AP5`): from a generated picture to the art pack, and the
//! record of what generation cost.
//!
//! - **Reduction.** A candidate becomes the fighter's pens: hard alpha,
//!   each opaque pixel the nearest of his opaque pens (never pen 15),
//!   cropped to what is opaque.
//! - **The ledger.** Every committed generation call is fully recorded, no
//!   month of the plan passes 90% of the allowance, and every generated
//!   pose a recipe names is a kept candidate in the ledger with the same
//!   checksum.
//! - **Derived poses (#170, AP6).** A pose made from another fighter's kept
//!   one, as Ken's throw is from Ryu's, names its source file and what was
//!   changed, and costs no generations.
//! - **Pens that share a colour (#175, AP11).** Geki's body blue (pen 14) is
//!   also his teleport smoke's colour (pen 6), so a picture cannot tell them
//!   apart; it goes to the pen his stance draws.

use std::path::Path;

use sf_art::ledger::{self, cycle, Ledger, CAP};
use sf_art::pack::{crop_to_opaque, default_anchor, reduce, stance_pens, Rgba};
use sf_engine::roster::TournamentFighter;
use sf_video::romset::load_assets;
use sf_video::art::{Pens, TRANSPARENT};

const COLOURS: [u32; 16] = [
    0xff000000, 0xff664400, 0xff996644, 0xffcc9977, 0xffddaa88, 0xffddccaa, 0xffbbbbbb, 0xff777777, 0xff999999,
    0xff555555, 0xff444444, 0xff440044, 0xff660099, 0xff990088, 0xffbb00bb, 0xff111111,
];

fn rgba(width: usize, pixels: &[[u8; 4]]) -> Rgba {
    Rgba { width, height: pixels.len() / width, bytes: pixels.concat() }
}

#[test]
fn a_candidate_is_reduced_to_his_pens_and_cropped() {
    const CLEAR: [u8; 4] = [0, 0, 0, 0];
    let picture = rgba(
        4,
        &[
            CLEAR, CLEAR, CLEAR, CLEAR,
            CLEAR, [0x66, 0x44, 0x00, 255], [0x00, 0x00, 0x00, 255], CLEAR,
            // #121212 is nearest his black, not the transparent pen's
            // #111111; a half-clear pixel is opaque, a mostly clear one
            // is not.
            CLEAR, [0x12, 0x12, 0x12, 255], [0xbb, 0x00, 0xbb, 128], [0xff, 0xff, 0xff, 127],
            CLEAR, CLEAR, CLEAR, CLEAR,
        ],
    );
    let r = reduce(&picture, &COLOURS, &[0; 16]).unwrap();
    assert_eq!(r.pens, Pens { width: 2, height: 2, pens: vec![1, 0, 0, 14] });
    assert_eq!((r.exact, r.nearest, r.soft_alpha), (3, 1, 2));
    assert!(!r.pens.pens.contains(&TRANSPARENT), "#121212 never becomes the transparent pen");
    assert!(reduce(&rgba(1, &[CLEAR]), &COLOURS, &[0; 16]).is_err(), "nothing opaque");

    // Its first anchor centres it on him with its lowest row on the ground.
    let tall = Pens { width: 63, height: 91, pens: vec![1; 63 * 91] };
    assert_eq!(default_anchor(&tall), [-31, -91]);
    let blank = Pens { width: 3, height: 3, pens: vec![TRANSPARENT; 9] };
    assert_eq!(crop_to_opaque(&blank).pens, Vec::<u8>::new());
}

/// Ticket #175 (`AP11`): where two of his pens share a colour, a pixel of
/// that colour (or nearest it) takes the one his stance draws more of; with
/// nothing to go on, the lower pen, as before.
#[test]
fn a_colour_two_pens_share_goes_to_the_pen_his_stance_draws() {
    let mut colours = COLOURS;
    colours[6] = 0xff6666bb;
    colours[14] = 0xff6666bb;
    let picture = rgba(2, &[[0x66, 0x66, 0xbb, 255], [0x67, 0x66, 0xbb, 255]]);
    let mut drawn = [0; 16];
    drawn[14] = 40;
    assert_eq!(reduce(&picture, &colours, &drawn).unwrap().pens.pens, [14, 14]);
    assert_eq!(reduce(&picture, &colours, &[0; 16]).unwrap().pens.pens, [6, 6]);
}

/// Geki's stance draws his body blue in pen 14 and never pen 6, his smoke's.
#[test]
fn gekis_stance_draws_his_blue_in_pen_14() {
    let roms = Path::new("../../tools/reference/local/roms/sf");
    if !roms.is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let rom = load_assets(roms, "../../tools/reference/rom-manifest.json").unwrap();
    let drawn = stance_pens(&rom, TournamentFighter::Geki);
    assert!(drawn[14] > 0 && drawn[6] == 0, "{drawn:?}");
    assert!(drawn[15] > 0, "his stance's grid has clear pixels too");
}

#[test]
fn a_call_falls_in_the_month_of_the_plan_it_was_made_in() {
    assert_eq!(cycle("2026-11-03T00:33:30Z").unwrap(), "2026-10-27");
    assert_eq!(cycle("2026-11-26T23:59:59Z").unwrap(), "2026-10-27");
    assert_eq!(cycle("2026-11-27T00:00:00Z").unwrap(), "2026-11-27");
    assert_eq!(cycle("2027-01-05T10:00:00Z").unwrap(), "2026-12-27");
    assert!(cycle("yesterday").is_err());
    assert_eq!(CAP, 2_000);
}

#[test]
fn the_committed_ledgers_are_complete_and_within_the_cap() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art");
    let read = |dir: &str| -> Vec<(String, String)> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(root.join(dir)).unwrap().flatten() {
            if e.path().extension().is_some_and(|x| x == "json") {
                out.push((e.path().display().to_string(), std::fs::read_to_string(e.path()).unwrap()));
            }
        }
        out
    };
    let ledgers: Vec<Ledger> = read("generations").iter().map(|(p, t)| serde_json::from_str(t).unwrap_or_else(|e| panic!("{p}: {e}"))).collect();
    ledger::check(&ledgers).unwrap();
    for (month, spent) in ledger::by_cycle(&ledgers).unwrap() {
        println!("the month from {month}: {spent} of {CAP}");
    }
    // Every generated pose in a recipe is a kept candidate with its
    // checksum, or derived from one.
    for (path, text) in read("recipes") {
        let recipe: serde_json::Value = serde_json::from_str(&text).unwrap();
        let fighter = recipe["fighter"].as_str().unwrap();
        for pose in recipe["poses"].as_array().unwrap() {
            let Some(crc) = pose["pack"]["crc32"].as_str() else { continue };
            let name = pose["name"].as_str().unwrap();
            let derived =
                ledgers.iter().filter(|l| l.fighter == fighter).flat_map(|l| &l.derived).any(|d| d.pose == name && d.crc32 == crc);
            assert!(
                ledger::kept(&ledgers, fighter, name, crc) || derived,
                "{path}: {fighter} {name} {crc} is generated but no ledger keeps or derives that file"
            );
        }
    }
}

#[test]
fn a_ledger_past_the_cap_or_missing_a_record_is_refused() {
    let call = |at: &str, cost: u32, references: &str| {
        format!(
            r##"{{ "at": "{at}", "move": "m", "tool": "t", "job": "j", "cost": {cost}, "canvas": [128, 128], "seed": 1,
                 "palette": ["#000000"], "references": [{references}], "prompt": "p", "candidates": 4 }}"##
        )
    };
    let rom_ref = r#"{ "rom": { "code": "0x2c00", "large": true, "rows": 3, "cols": 1 }, "usage": "u" }"#;
    let ledger = |calls: &[String]| -> Ledger {
        serde_json::from_str(&format!(r#"{{ "fighter": "f", "calls": [{}] }}"#, calls.join(","))).unwrap()
    };
    let within = ledger(&[call("2026-11-03T00:00:00Z", 1_000, rom_ref), call("2026-11-26T00:00:00Z", 1_000, rom_ref)]);
    ledger::check(&[within]).unwrap();
    let past = ledger(&[call("2026-11-03T00:00:00Z", 1_000, rom_ref), call("2026-11-26T00:00:00Z", 1_010, rom_ref)]);
    assert!(ledger::check(&[past]).unwrap_err().contains("past the cap of 2000"));
    // The next month starts afresh on the 27th.
    let next = ledger(&[call("2026-11-26T00:00:00Z", 2_000, rom_ref), call("2026-11-27T00:00:00Z", 2_000, rom_ref)]);
    ledger::check(&[next]).unwrap();
    let neither = ledger(&[call("2026-11-03T00:00:00Z", 10, r#"{ "usage": "u" }"#)]);
    assert!(ledger::check(&[neither]).unwrap_err().contains("must be a ROM pose or a candidate"));
    let free = ledger(&[call("2026-11-03T00:00:00Z", 0, rom_ref)]);
    assert!(ledger::check(&[free]).unwrap_err().contains("must all be recorded"));
}

#[test]
fn a_derived_pose_must_come_from_a_kept_one() {
    let source: Ledger = serde_json::from_str(
        r##"{ "fighter": "ryu", "calls": [{ "at": "2026-11-03T00:00:00Z", "move": "throw", "tool": "t", "job": "j", "cost": 10,
             "canvas": [108, 108], "seed": 1, "palette": ["#000000"], "prompt": "p", "candidates": 4,
             "references": [{ "rom": { "code": "0x0063", "large": true, "rows": 2, "cols": 1 }, "usage": "u" }],
             "kept": [{ "candidate": 2, "pose": "throw_turn", "crc32": "cc3890e5" }] }] }"##,
    )
    .unwrap();
    let derived = |crc: &str, touch_ups: &str| -> Ledger {
        serde_json::from_str(&format!(
            r#"{{ "fighter": "ken", "calls": [], "derived": [{{ "pose": "throw_turn", "crc32": "8ae7a3bb",
                 "from": {{ "fighter": "ryu", "pose": "throw_turn", "crc32": "{crc}" }}, "touch_ups": [{touch_ups}] }}] }}"#
        ))
        .unwrap()
    };
    ledger::check(&[source.clone(), derived("cc3890e5", r#""recoloured""#)]).unwrap();
    // From a file no call kept, or with nothing said of what changed.
    let stale = ledger::check(&[source.clone(), derived("3ba39388", r#""recoloured""#)]).unwrap_err();
    assert!(stale.contains("which no call kept"), "{stale}");
    let silent = ledger::check(&[source, derived("cc3890e5", "")]).unwrap_err();
    assert!(silent.contains("must be recorded"), "{silent}");
}
