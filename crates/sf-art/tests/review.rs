//! Ticket #168 (`AP4`): the review page.
//!
//! - **The decisions.** Approving and rejecting follow the design's rules:
//!   a rejected free pose is generated instead; a rejected generated pose
//!   is retried until the third rejection hands it back to the user; the
//!   fighter is approved only when every pose is. Verdicts survive the
//!   committed file.
//! - **The frames.** A pose placed at its anchor through the engine's grid
//!   lands where the game draws it: Ryu's stance at its anchor reproduces
//!   the real fight frame pixel for pixel.
//! - **The page.** It serves every move and frame and saves verdicts.
//!
//! The frame and page tests need the local ROMs and skip without them.

use std::path::Path;

use sf_video::art_review::{Next, Origin, Review, Verdict};
use sf_art::frame::fight_frame;
use sf_art::site::{bmp, Site};
use sf_engine::roster::TournamentFighter;
use sf_video::art::{load_art, rom_only, Catalogue, Grid, Move, Pose, Source};
use sf_video::render::{render_frame, HEIGHT, WIDTH};
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const RYU: &str = include_str!("../../../art/recipes/ryu.json");

fn catalogue() -> Catalogue {
    let pose = |name: &str| Pose {
        name: name.to_owned(),
        source: Source::Rom,
        grid: Grid { code: 0x63, large: true, rows: 2, cols: 1 },
        anchor: [-32, -96],
        pack: None,
        note: String::new(),
    };
    Catalogue {
        fighter: "test".to_owned(),
        poses: vec![pose("a"), pose("b")],
        moves: vec![Move { name: "m".to_owned(), poses: vec!["a".to_owned(), "b".to_owned()], timing: vec![10, 10], note: String::new() }],
    }
}

#[test]
fn verdicts_follow_the_designs_rules() {
    let mut r = Review::for_catalogue(&catalogue(), None);
    assert_eq!(r.poses.len(), 2);
    assert!(r.poses.values().all(|p| p.verdict == Verdict::Pending && p.origin == Origin::Free));
    assert!(!r.approved());
    // A rejected free pose goes to generation.
    assert_eq!(r.decide("a", false).unwrap().next, Next::Generate);
    r.decide("a", true).unwrap();
    assert!(!r.approved(), "b is still pending");
    r.decide("b", true).unwrap();
    assert!(r.approved(), "every pose approved");
    assert!(r.decide("nope", true).is_err());
    // A generated pose: retry, retry, then the user decides.
    r.poses.get_mut("b").unwrap().origin = Origin::Generated;
    assert_eq!(r.decide("b", false).unwrap().next, Next::Retry);
    assert_eq!(r.decide("b", false).unwrap().next, Next::Retry);
    let third = r.decide("b", false).unwrap();
    assert_eq!((third.rejections, third.next), (3, Next::UserDecides));
    assert!(!r.approved());
}

#[test]
fn a_verdict_holds_for_the_generated_file_it_was_given_on() {
    let mut cat = catalogue();
    cat.poses[1].source = Source::Pack;
    cat.poses[1].pack = Some("11111111".to_owned());
    let mut r = Review::for_catalogue(&cat, None);
    assert_eq!((r.poses["b"].origin, r.poses["b"].crc32.as_deref()), (Origin::Generated, Some("11111111")));
    r.decide("a", true).unwrap();
    r.decide("b", false).unwrap();
    // The retry is a new file: pending again, its rejection still counted.
    cat.poses[1].pack = Some("22222222".to_owned());
    let retry = Review::for_catalogue(&cat, Some(&r));
    let b = &retry.poses["b"];
    assert_eq!((b.verdict, b.rejections, b.next, b.crc32.as_deref()), (Verdict::Pending, 1, Next::None, Some("22222222")));
    assert_eq!(retry.poses["a"].verdict, Verdict::Approved, "a free pose keeps its verdict");
    // An approval does not carry over to another file either.
    let mut approved = retry.clone();
    approved.decide("b", true).unwrap();
    assert!(approved.approved());
    cat.poses[1].pack = Some("33333333".to_owned());
    assert!(!Review::for_catalogue(&cat, Some(&approved)).approved());
    // The same file keeps it.
    cat.poses[1].pack = Some("22222222".to_owned());
    assert!(Review::for_catalogue(&cat, Some(&approved)).approved());
}

#[test]
fn verdicts_survive_the_file_and_a_new_catalogue() {
    let dir = std::env::temp_dir().join(format!("sf1-port-review-{}", std::process::id()));
    let path = dir.join("test.json");
    let mut r = Review::for_catalogue(&catalogue(), None);
    r.decide("a", true).unwrap();
    r.save(&path).unwrap();
    let back = Review::load(&path).unwrap();
    assert_eq!(back, r);
    // A catalogue that gains a pose keeps the old verdicts and adds it pending.
    let mut more = catalogue();
    let mut c = more.poses[0].clone();
    c.name = "c".to_owned();
    more.poses.push(c);
    more.moves[0].poses.push("c".to_owned());
    let merged = Review::for_catalogue(&more, Some(&back));
    assert_eq!(merged.poses["a"].verdict, Verdict::Approved);
    assert_eq!(merged.poses["c"].verdict, Verdict::Pending);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_pose_at_its_anchor_lands_where_the_game_draws_it() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let rom = load_assets(ROMS, MANIFEST).unwrap();
    let art = load_art(&rom, &[&rom_only(RYU).unwrap()]).unwrap();
    let ryu = art.catalogue("ryu").unwrap();
    let ff = fight_frame(&rom, TournamentFighter::Ryu, TournamentFighter::Ken).unwrap();
    let banked = rom.with_sprite_bank(&art.bank);
    let real = render_frame(&banked, &ff.scene);
    // He stands in his stance in the real frame: drawing the catalogue's
    // stance at its anchor must reproduce the frame exactly.
    let redrawn = render_frame(&banked, &ff.with_pose(ryu.pose("stance").unwrap(), false));
    assert_eq!(redrawn.argb, real.argb, "the stance anchor puts him exactly where the game does");
    // Another pose changes the frame only around him.
    let lying = render_frame(&banked, &ff.with_pose(ryu.pose("lying").unwrap(), false));
    let changed: Vec<usize> = (0..real.argb.len()).filter(|&i| lying.argb[i] != real.argb[i]).collect();
    assert!(!changed.is_empty());
    let xs = changed.iter().map(|i| (i % WIDTH) as i32);
    let (lo, hi) = (xs.clone().min().unwrap(), xs.max().unwrap());
    println!("centre {}, ground {}; lying changes columns {lo}..={hi}", ff.centre_x, ff.ground_y);
    assert!(hi < (WIDTH / 2) as i32, "the change stays on his half of the screen");
}

#[test]
fn the_page_serves_every_move_and_saves_verdicts() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let rom = load_assets(ROMS, MANIFEST).unwrap();
    let art = load_art(&rom, &[&rom_only(RYU).unwrap()]).unwrap();
    let dir = std::env::temp_dir().join(format!("sf1-port-site-{}", std::process::id()));
    let path = dir.join("ryu.json");
    let mut site = Site::build(&rom, &art, TournamentFighter::Ryu, path.clone()).unwrap();
    let (status, kind, page) = site.handle("GET", "/", "");
    let page = String::from_utf8(page).unwrap();
    assert_eq!((status, kind), (200, "text/html; charset=utf-8"));
    assert!(page.contains("\"get-up\"") && page.contains("\"thrown\"") && page.contains("[60,12,10,60]"));
    for f in ["/rom.bmp", "/pose/rising.bmp", "/pose/rising-left.bmp", "/pose/thrown_1.bmp"] {
        let (status, kind, body) = site.handle("GET", f, "");
        assert_eq!((status, kind, body.len()), (200, "image/bmp", 54 + WIDTH * 3 * HEIGHT), "{f}");
    }
    assert_eq!(site.handle("GET", "/pose/nope.bmp", "").0, 404);
    let (status, _, state) = site.handle("POST", "/decide", "pose=rising&verdict=approve");
    assert_eq!(status, 200);
    assert!(String::from_utf8(state).unwrap().contains("\"approved\":false"));
    let saved = Review::load(&path).unwrap();
    assert_eq!(saved.poses["rising"].verdict, Verdict::Approved);
    assert_eq!(saved.poses["lying"].verdict, Verdict::Pending);
    assert_eq!(site.handle("POST", "/decide", "pose=nope&verdict=approve").0, 404);
    // The BMP header is right.
    let b = bmp(&render_frame(&rom, &fight_frame(&rom, TournamentFighter::Ryu, TournamentFighter::Ken).unwrap().scene));
    assert_eq!(&b[..2], b"BM");
    let _ = std::fs::remove_dir_all(dir);
}
