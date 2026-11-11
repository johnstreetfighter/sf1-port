//! Ticket #167 (`AP3`): the art-loading seam with recipes.
//!
//! Ryu's recipe file gives his get-up from ROM poses alone (lying, rising,
//! crouch, stance) and his being-thrown set (AP6), two of its poses
//! rebuilt into the bank. His generated throw needs the local art pack, so
//! these tests build his recipe from his ROM alone. They check:
//! - the catalogue's order, sources, places and anchors;
//! - that the build is identical every time;
//! - that every pose keeps Ryu's own pens (pen 15 transparent) and fits 24
//!   sprite entries;
//! - that a broken recipe is a reason to fall back, never a panic.
//!
//! Local ROMs only; each test skips without them.

use std::path::Path;

use sf_video::art::{load_art, rom_only, Pens, Source, MAX_ENTRIES, TRANSPARENT};
use sf_video::romset::{load_assets, Assets};
use sf_video::sprite_bank::ROM_SPRITE_TILES;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const RYU: &str = include_str!("../../../art/recipes/ryu.json");

/// Ryu's recipe from his ROM alone: his generated poses need the local art
/// pack, which these checks do without.
fn ryu() -> String {
    rom_only(RYU).unwrap()
}

fn rom() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).unwrap())
}

#[test]
fn ryus_catalogue_lists_his_get_up_and_his_thrown_set() {
    let Some(a) = rom() else { return };
    let art = load_art(&a, &[&ryu()]).expect("Ryu's recipes build");
    let ryu = art.catalogue("ryu").unwrap();
    let get_up: Vec<_> = ryu.move_poses("get-up").unwrap();
    let names: Vec<&str> = get_up.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["lying", "rising", "crouch", "stance"]);
    assert!(get_up.iter().all(|p| p.source == Source::Rom), "the get-up needs no new tiles");
    let stance = ryu.pose("stance").unwrap();
    assert_eq!((stance.grid.code, stance.grid.large, stance.grid.rows, stance.grid.cols), (0x0063, true, 2, 1));
    assert_eq!(stance.anchor, [-32, -96], "64 wide and 96 tall, centred, feet on the ground line");
    assert_eq!(ryu.pose("lying").unwrap().anchor, [-64, -32], "centred: he rises where he lies");
    let thrown = ryu.pose("thrown_1").unwrap();
    assert_eq!(thrown.source, Source::Recipe);
    assert_eq!(thrown.grid.code as usize, ROM_SPRITE_TILES, "the bank's first rebuilt pose");
    assert_eq!((thrown.grid.large, thrown.grid.rows, thrown.grid.cols), (false, 3, 3), "64x64 in 16-pixel entries");
    // His being-thrown set (AP6) is all his own art: two ROM poses and two
    // rebuilt from his fall pose, turned a quarter and a half.
    let thrown: Vec<&str> = ryu.move_poses("thrown").unwrap().iter().map(|p| p.name.as_str()).collect();
    assert_eq!(thrown, ["grabbed", "thrown_1", "thrown_2", "lying"]);
    let over = ryu.pose("thrown_2").unwrap();
    assert_eq!((over.source.clone(), over.grid.code), (Source::Recipe, 0x3840), "the next four sheet rows");
    // The generated throw needs the art pack, so the ROM-only recipe has none.
    assert!(ryu.move_poses("throw").is_none());
    // Each rebuilt pose is four sheet rows of four tiles, and the swizzle
    // puts each pose's fourth row in the slots its third leaves free
    // (0x3830..0x3833 in 0x3828..0x382b): two poses span 108 slots.
    assert_eq!(art.bank.tiles(), 0x6c);
    println!("{}", art.describe());
}

#[test]
fn rebuilding_the_recipes_gives_identical_tiles() {
    let Some(a) = rom() else { return };
    assert_eq!(load_art(&a, &[&ryu()]).unwrap(), load_art(&a, &[&ryu()]).unwrap());
}

#[test]
fn every_pose_keeps_ryus_pens_and_fits_the_object_region() {
    let Some(a) = rom() else { return };
    let art = load_art(&a, &[&ryu()]).unwrap();
    let ryu = art.catalogue("ryu").unwrap();
    for pose in &ryu.poses {
        assert!(pose.grid.entries() <= MAX_ENTRIES, "{}", pose.name);
    }
    // The rebuilt pose holds exactly the pens of the ROM pose it was turned
    // from, with the same count of each, and is transparent elsewhere.
    let (_, _, built) = art.built.iter().find(|(f, n, _)| f == "ryu" && n == "thrown_1").unwrap();
    let fall = ryu.pose("rising").unwrap().grid;
    let source = Pens::from_grid(&a.gfx3, &fall);
    let count = |p: &Pens| {
        let mut c = [0usize; 16];
        for &pen in &p.pens {
            c[pen as usize] += 1;
        }
        c
    };
    assert_eq!(count(built), count(&source));
    assert_eq!((built.width, built.height), (64, 64));
    assert!(built.pens.contains(&TRANSPARENT));
    // A quarter turn clockwise: the source's bottom-left lands top-left.
    assert_eq!(built.at(0, 0), source.at(0, 63));
    assert_eq!(built.at(63, 0), source.at(0, 0));
}

#[test]
fn a_broken_recipe_is_a_reason_to_fall_back() {
    let Some(a) = rom() else { return };
    let broken = [
        ("not json", "{"),
        ("unknown field", r#"{"fighter":"x","poses":[],"moves":[],"extra":1}"#),
        ("past the ROM", r#"{"fighter":"x","poses":[{"name":"p","rom":{"code":"0x37ff","large":true,"rows":0,"cols":0},"anchor":[0,0]}],"moves":[]}"#),
        ("too many entries", r#"{"fighter":"x","poses":[{"name":"p","rom":{"code":"0x0000","large":false,"rows":4,"cols":4},"anchor":[0,0]}],"moves":[]}"#),
        ("a move with no pose", r#"{"fighter":"x","poses":[],"moves":[{"name":"m","poses":["nope"]}]}"#),
        ("neither ROM nor parts", r#"{"fighter":"x","poses":[{"name":"p","anchor":[0,0]}],"moves":[]}"#),
    ];
    for (what, text) in broken {
        let err = load_art(&a, &[text]).expect_err(what);
        println!("{what}: {err}");
    }
}
