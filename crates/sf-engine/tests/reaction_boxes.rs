//! Ticket #261 (`SP17`): the hurtboxes of the hit reactions true combos
//! make hittable.
//!
//! In 1987 every reaction frame has a zero-size hurtbox and the invulnerable
//! flag; the combo gate writes real boxes for the ground reaction frames,
//! from `sf_engine::combo::reaction_boxes`, which `sf_engine::combo::derive`
//! measures from the ROMs. These tests check that every box the gate writes
//! passes the ROM box audit's own thresholds (`docs/reference/rom-box-audit.md`:
//! the design's rule that a box this project writes matches what is drawn),
//! that every ground reaction frame of the twelve fighters has one, and that
//! the committed table is what the ROMs give. To rewrite the table:
//! `UPDATE_REACTION_BOXES=1 cargo test --release -p sf-engine --test reaction_boxes`
//! (PowerShell: `$env:UPDATE_REACTION_BOXES=1; cargo test ...`).
//!
//! Every test skips without the local ROM set.

use std::path::Path;

use sf_engine::box_audit::{BODY_EXCESS_FLAG, BODY_OVERREACH_FLAG, TIGHT_OUTSIDE_FLAG};
use sf_engine::combo::derive::{all_reaction_boxes, render_table, Source};
use sf_engine::combo::frame_boxes;
use sf_engine::roster::TournamentFighter;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const TABLE: &str = "src/combo/reaction_boxes.rs";

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

#[test]
fn every_box_passes_the_audits_thresholds() {
    let Some(a) = assets() else { return };
    let boxes = all_reaction_boxes(&a.maincpu, &a.gfx3);
    let mut worst = (0.0f64, 0i32, 0i32);
    for b in &boxes {
        let what = format!("{} frame 0x{:05x} ({:?})", b.fighter.name(), b.frame, b.source);
        assert!(!b.loose.is_empty(), "{what}: a real hurtbox");
        assert!(b.excess < BODY_EXCESS_FLAG, "{what}: {:.0}% more of the drawing outside than the stance", b.excess * 100.0);
        assert!(b.overreach < BODY_OVERREACH_FLAG, "{what}: the hurtbox lies {} px past the drawing", b.overreach);
        assert!(b.tight_out < TIGHT_OUTSIDE_FLAG, "{what}: the critical box sticks {} px out", b.tight_out);
        worst = (worst.0.max(b.excess), worst.1.max(b.overreach), worst.2.max(b.tight_out));
    }
    let drawn = boxes.iter().filter(|b| b.source == Source::Drawn).count();
    eprintln!(
        "{} frames ({} placed on the drawing); worst: {:+.0} points outside, {} px past, {} px critical overhang",
        boxes.len(),
        drawn,
        worst.0 * 100.0,
        worst.1,
        worst.2
    );
}

#[test]
fn every_fighters_ground_reactions_have_boxes() {
    let Some(a) = assets() else { return };
    let boxes = all_reaction_boxes(&a.maincpu, &a.gfx3);
    for f in TournamentFighter::ALL {
        let mine: Vec<_> = boxes.iter().filter(|b| b.fighter == f).collect();
        assert!(!mine.is_empty(), "{} has ground reactions", f.name());
        for b in mine {
            assert_eq!(frame_boxes(b.frame), Some(b.bytes()), "{} frame 0x{:05x} is in the engine's table", f.name(), b.frame);
        }
    }
    // Ryu's standing reaction ends on his stance pose, which keeps its 1987 boxes.
    let stance = boxes.iter().find(|b| b.frame == 0x01bc2).expect("Ryu's 0x30 ends at 0x01bc2");
    assert_eq!(stance.source, Source::Reference);
}

#[test]
fn the_committed_table_is_current() {
    let Some(a) = assets() else { return };
    let text = render_table(&all_reaction_boxes(&a.maincpu, &a.gfx3));
    if std::env::var_os("UPDATE_REACTION_BOXES").is_some() {
        std::fs::write(TABLE, &text).expect("write the table");
        return;
    }
    let committed = std::fs::read_to_string(TABLE).expect("the committed table").replace("\r\n", "\n");
    assert!(committed == text, "{TABLE} is stale: rerun with UPDATE_REACTION_BOXES=1");
}
