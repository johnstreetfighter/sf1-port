//! Ticket #250 (`SP16`): the mirror palettes' review.
//!
//! - **The palettes.** All twelve are committed, picked from the fighter's
//!   own colours, and read against his normal palette.
//! - **The verdicts.** Approve or reject with a reason, kept per fighter
//!   and good for the colours they were given on.
//! - **The page.** It serves each fighter's pictures and records verdicts.
//!
//! The palette checks and the page need the local ROMs and skip without
//! them.

use std::path::Path;

use sf_art::frame::fight_frame;
use sf_art::palette_site::{parse_form, PaletteSite};
use sf_art::palettes::{changed_share, mirror_colours, own_colours, palette_colours, review_path, rom_palette, PaletteReview, Verdict};
use sf_art::site::Responder;
use sf_engine::roster::TournamentFighter;
use sf_video::art::{load_art, rom_only, Grid, Pens};
use sf_video::mirror_palette::{self, MirrorPalette};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn mirror(f: TournamentFighter) -> &'static MirrorPalette {
    mirror_palette::for_fighter(f.name()).expect("a committed mirror palette")
}

fn art_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art")
}

/// The stance's grid: the header pose of move 1, as the pipeline reads it.
fn stance(rom: &Assets, f: TournamentFighter) -> Grid {
    let m = &rom.maincpu;
    let h = f.move_table() as usize + 0x30;
    let word = |a: usize| u16::from_be_bytes([m[a], m[a + 1]]);
    Grid { code: word(h), large: (word(h + 2) as i16) >= 0x400, rows: m[h + 0xc], cols: m[h + 0xd] }
}

// ---- the palettes --------------------------------------------------------------

#[test]
fn all_twelve_are_committed_and_each_names_the_fighters_own_palette() {
    assert_eq!(mirror_palette::committed().len(), 12);
    for f in TournamentFighter::ALL {
        let m = mirror(f);
        assert!(!m.note.is_empty(), "{f:?} says what his palette is");
        assert_eq!(m.from, f.sprite_palette(), "{f:?}");
        assert_eq!(m.slot, 14, "{f:?}: the free palette the gate uses");
    }
}

#[test]
fn a_fighters_own_palette_is_the_one_his_fight_frames_draw_him_in() {
    let Some(rom) = assets() else { return };
    for f in TournamentFighter::ALL {
        let opponent = if f == TournamentFighter::Ryu { TournamentFighter::Ken } else { TournamentFighter::Ryu };
        let frame = fight_frame(&rom, f, opponent).expect("a fight frame");
        assert_eq!(frame.palette, u16::from(f.sprite_palette()), "{f:?}");
    }
}

/// The pens whose colours the maintainer chose himself, because his ROM palette has
/// none that would do (2026-11-08): Retsu's saffron robe with cream and ochre under it (pens 6 and 8 to 11) and
/// Eagle's navy trousers (pens 11 to 14). Every other pen of every mirror
/// palette is one of the fighter's own colours.
const CHOSEN: [(TournamentFighter, &[usize]); 2] =
    [(TournamentFighter::Retsu, &[6, 8, 9, 10, 11]), (TournamentFighter::Eagle, &[11, 12, 13, 14])];

#[test]
fn every_mirror_colour_is_one_of_the_fighters_own_but_the_two_approved_exceptions() {
    let Some(rom) = assets() else { return };
    for f in TournamentFighter::ALL {
        let own = own_colours(&rom, f);
        let chosen: &[usize] = CHOSEN.iter().find(|(c, _)| *c == f).map_or(&[], |(_, pens)| pens);
        for (pen, word) in mirror(f).pens.iter().enumerate() {
            if chosen.contains(&pen) {
                assert!(!own.contains(word), "{f:?}: pen {pen} is one of his own after all; take it off the chosen list");
            } else {
                assert!(own.contains(word), "{f:?}: pen {pen} is {word:03x}, none of his colours");
            }
        }
    }
}

#[test]
fn the_mirror_keeps_his_outline_and_skin_and_reads_against_the_normal_palette() {
    let Some(rom) = assets() else { return };
    for f in TournamentFighter::ALL {
        let m = mirror(f);
        let normal = rom_palette(&rom, f.sprite_palette());
        // His outline (pen 0) and his skin (pens 1 to 5) are what P1 and P2
        // share: they are both the same fighter. Adon, mostly skin and
        // black, changes least (about a fifth of his stance).
        for pen in 0..6 {
            assert_eq!(m.pens[pen], normal[pen], "{f:?} pen {pen}");
        }
        let pens = Pens::from_grid(&rom.gfx3, &stance(&rom, f));
        let share = changed_share(&pens, &palette_colours(&rom, f.sprite_palette()), &mirror_colours(&rom, m), 80.0);
        println!("{f:?}: {:.0}% of his stance is clearly another colour", share * 100.0);
        assert!(share >= 0.15, "{f:?}: only {:.0}% of his stance changes", share * 100.0);
    }
}

// ---- the verdicts ----------------------------------------------------------------

fn palette(word: u16) -> MirrorPalette {
    let colour = mirror_palette::colour_text(word);
    let pens = vec![colour; 15];
    MirrorPalette::parse(
        &serde_json::json!({ "fighter": "ryu", "note": "n", "from": 0, "slot": 14, "pens": pens }).to_string(),
    )
    .expect("a palette")
}

#[test]
fn approve_and_reject_follow_the_design_and_a_rejection_needs_a_reason() {
    let p = palette(0x123);
    let mut r = PaletteReview::pending(&p);
    assert_eq!((r.verdict, r.rejections), (Verdict::Pending, 0));
    assert!(!r.approved_for(&p));
    assert!(r.decide(&p, false, "  ").unwrap_err().contains("reason"), "a rejection with no reason is refused");
    assert_eq!(r.verdict, Verdict::Pending);
    r.decide(&p, false, "too close to P1's red").unwrap();
    assert_eq!((r.verdict, r.rejections, r.reason.as_str()), (Verdict::Rejected, 1, "too close to P1's red"));
    r.decide(&p, true, "").unwrap();
    assert!(r.approved_for(&p), "approval needs no reason");
    assert_eq!((r.history.len(), r.rejections), (2, 1), "both decisions are kept");
    assert_eq!(r.history[0].reason, "too close to P1's red");
}

#[test]
fn a_verdict_holds_for_the_colours_it_was_given_on() {
    let p = palette(0x123);
    let mut r = PaletteReview::pending(&p);
    r.decide(&p, true, "fine").unwrap();
    // The same colours: the verdict stands. Other colours: pending again,
    // with what was said kept.
    assert_eq!(PaletteReview::current(Some(r.clone()), &p), r);
    let changed = palette(0x456);
    assert_ne!(p.digest(), changed.digest());
    let again = PaletteReview::current(Some(r.clone()), &changed);
    assert_eq!((again.verdict, again.palette.as_str()), (Verdict::Pending, changed.digest().as_str()));
    assert!(!again.approved_for(&changed));
    assert_eq!(again.history, r.history);
    assert!(!r.approved_for(&changed), "the old verdict does not approve new colours");
}

#[test]
fn verdicts_survive_the_committed_file() {
    let dir = std::env::temp_dir().join(format!("sf-art-palette-review-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let p = palette(0x123);
    let path = review_path(&dir, "ryu");
    let mut r = PaletteReview::pending(&p);
    r.decide(&p, false, "too dark; headband lost").unwrap();
    r.save(&path).unwrap();
    assert_eq!(PaletteReview::load(&path), Some(r));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_committed_verdicts_name_their_fighters_and_never_stand_for_other_colours() {
    for f in TournamentFighter::ALL {
        let name = f.name().to_ascii_lowercase();
        let path = review_path(&art_dir(), &name);
        let review = PaletteReview::load(&path).unwrap_or_else(|| panic!("{}: a verdict file", path.display()));
        assert_eq!(review.fighter, name);
        // An approval must be for the colours now committed; one given on
        // other colours is stale (the page makes it pending when it starts).
        if review.verdict == Verdict::Approved {
            assert!(review.approved_for(mirror(f)), "{name}: approved on other colours");
        }
        if review.verdict == Verdict::Rejected {
            assert!(!review.reason.is_empty(), "{name}: rejected with no reason");
        }
    }
}

#[test]
fn form_bodies_decode_reasons_with_spaces_and_symbols() {
    let form = parse_form("fighter=ryu&verdict=reject&reason=too+close+to+P1%27s+red%2C+%26+dark");
    assert_eq!(form["reason"], "too close to P1's red, & dark");
    assert_eq!(form["fighter"], "ryu");
}

// ---- the page ----------------------------------------------------------------------

fn build_site(rom: &Assets, review_dir: &Path) -> PaletteSite {
    let recipes: Vec<String> = TournamentFighter::ALL
        .iter()
        .map(|f| std::fs::read_to_string(art_dir().join("recipes").join(format!("{}.json", f.name().to_ascii_lowercase()))).unwrap())
        .map(|t| rom_only(&t).unwrap())
        .collect();
    let refs: Vec<&str> = recipes.iter().map(String::as_str).collect();
    let art = load_art(rom, &refs).expect("the recipes build without the pack");
    PaletteSite::build(rom, Some(&art), review_dir).expect("the page builds")
}

#[test]
fn the_page_serves_every_fighter_with_pictures_and_records_verdicts_with_reasons() {
    let Some(rom) = assets() else { return };
    let dir = std::env::temp_dir().join(format!("sf-art-palette-page-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut site = build_site(&rom, &dir);
    let (status, kind, page) = site.respond("GET", "/", "");
    let page = String::from_utf8(page).unwrap();
    assert_eq!((status, kind), (200, "text/html; charset=utf-8"));
    for f in TournamentFighter::ALL {
        let name = f.name().to_ascii_lowercase();
        assert!(page.contains(&format!("\"name\":\"{name}\"")), "{name} is on the page");
        // His poses, effects and fight frames are served as PNGs.
        for what in ["pose-stance", "fight-standing", "fight-punching"] {
            let (status, kind, png) = site.respond("GET", &format!("/img/{name}/{what}.png"), "");
            assert_eq!((status, kind), (200, "image/png"), "{name} {what}");
            assert_eq!(&png[1..4], b"PNG");
        }
    }
    let (status, _, _) = site.respond("GET", "/img/ryu/nothing.png", "");
    assert_eq!(status, 404);
    // Only the body has a mirror palette: the page shows no effect rows.
    let (status, _, _) = site.respond("GET", "/img/ryu/effect-Hadoken--first-frame.png", "");
    assert_eq!(status, 404);
    assert!(!page.contains("\"effects\""), "no effects on the page");

    // A rejection needs a reason; with one it is kept; an approval follows.
    let (status, _, why) = site.respond("POST", "/decide", "fighter=ryu&verdict=reject&reason=");
    assert_eq!(status, 400, "{}", String::from_utf8_lossy(&why));
    let (status, _, state) = site.respond("POST", "/decide", "fighter=ryu&verdict=reject&reason=too+like+Ken");
    assert_eq!(status, 200);
    let state: serde_json::Value = serde_json::from_slice(&state).unwrap();
    assert_eq!(state["fighters"]["ryu"]["verdict"], "rejected");
    assert_eq!(state["fighters"]["ryu"]["reason"], "too like Ken");
    assert_eq!(state["approved"], 0);
    let (status, _, state) = site.respond("POST", "/decide", "fighter=ryu&verdict=approve&reason=");
    assert_eq!(status, 200);
    let state: serde_json::Value = serde_json::from_slice(&state).unwrap();
    assert_eq!(state["approved"], 1);
    let (status, _, _) = site.respond("POST", "/decide", "fighter=nobody&verdict=approve&reason=");
    assert_eq!(status, 404);

    // The verdict is in the fighter's file, and a new page starts from it.
    let saved = PaletteReview::load(&review_path(&dir, "ryu")).unwrap();
    assert_eq!((saved.verdict, saved.history.len()), (Verdict::Approved, 2));
    let again = build_site(&rom, &dir);
    assert_eq!(again.review("ryu").unwrap().verdict, Verdict::Approved);
    assert_eq!(again.review("ken").unwrap().verdict, Verdict::Pending);
    let _ = std::fs::remove_dir_all(&dir);
}
