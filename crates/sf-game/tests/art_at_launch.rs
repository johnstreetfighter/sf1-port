//! Ticket #167 (`AP3`): the game builds its art at launch from the embedded
//! recipes and the player's ROM, logs what it loaded, and gives overhaul
//! fights the bank. A recipe that fails to build is logged and the game
//! keeps the ROM's own graphics.
//!
//! Ticket #169 (`AP5`): the art pack is looked for beside the ROM folders.
//! A fighter whose generated poses the pack does not hold, or whose poses
//! are not all approved, keeps his ROM art, and the log says why.
//!
//! Local ROMs only; each test that loads art skips without them.

use std::path::{Path, PathBuf};

use sf_host::art::{find_pack, load_from, pack_folders, RECIPES, REVIEWS};
use sf_host::log::Log;
use sf_video::art::rom_only;
use sf_video::romset::load_assets;
use sf_video::sprite_bank::ROM_SPRITE_TILES;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

/// The embedded recipe for `fighter`.
fn recipe(fighter: &str) -> &'static str {
    RECIPES.iter().find(|r| r.contains(&format!("\"fighter\": \"{fighter}\""))).unwrap()
}

fn scratch_log(name: &str) -> (Log, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("sf1-port-art-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("sf-game.log");
    (Log::create(&path), path)
}

#[test]
fn the_game_builds_its_art_at_launch_and_logs_it() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let rom = load_assets(ROMS, MANIFEST).unwrap();
    let (mut log, path) = scratch_log("ok");
    // With no pack (whatever this machine holds), every fighter with a
    // generated pose keeps his ROM art, logged with the reason: Ryu and Ken
    // (their throw), Sagat (his crouch), Retsu (his crouching normals), Geki
    // (his standing block) and Joe (his crouch).
    let art = load_from(&rom, &RECIPES, &REVIEWS, Err("no pack in this test".to_owned()), &mut log).expect("the embedded recipes build");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("art: no fighter's new poses are ready"), "{text}");
    for who in ["ryu", "ken", "sagat", "retsu", "geki", "joe"] {
        assert!(text.contains(&format!("art: {who} keeps his ROM art: ")), "{text}");
    }
    assert!(art.catalogues.is_empty() && art.bank.is_empty());
    // His art from his ROM alone, approved in his committed review, loads
    // into the bank: the get-up and the being-thrown set.
    let (mut log, path) = scratch_log("rom-only");
    let ryu = rom_only(recipe("ryu")).unwrap();
    let art = load_from(&rom, &[&ryu], &REVIEWS, Err("no pack in this test".to_owned()), &mut log).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("art: ryu (2 moves; 6 ROM poses, 2 rebuilt); the bank spans 108 tile slots"),
        "{text}"
    );
    assert_eq!(rom.with_sprite_bank(&art.bank).sprite_tiles(), ROM_SPRITE_TILES + 108);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn a_recipe_that_fails_keeps_the_roms_own_graphics() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let rom = load_assets(ROMS, MANIFEST).unwrap();
    let (mut log, path) = scratch_log("broken");
    assert!(load_from(&rom, &["{"], &REVIEWS, Err("none".to_owned()), &mut log).is_none());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("art: keeping the ROM's own graphics: a recipe file:"), "{text}");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn the_pack_is_looked_for_beside_each_rom_folder_once() {
    let a = PathBuf::from("game").join("roms").join("sf");
    let b = PathBuf::from("appdata").join("roms").join("sf");
    let folders = pack_folders(&[a.as_path(), b.as_path(), a.as_path()]);
    assert_eq!(folders, [PathBuf::from("game/roms/sf1-port-art"), PathBuf::from("appdata/roms/sf1-port-art")]);
    let none = std::env::temp_dir().join(format!("sf1-port-no-pack-{}", std::process::id()));
    assert_eq!(find_pack(std::slice::from_ref(&none)).unwrap_err(), "there is no sf1-port-art folder beside the ROMs");
    // A folder that is there but broken is the pack, refused with its reason.
    std::fs::create_dir_all(&none).unwrap();
    std::fs::write(none.join("manifest.json"), "not json").unwrap();
    assert!(find_pack(std::slice::from_ref(&none)).unwrap_err().contains("manifest.json"));
    let _ = std::fs::remove_dir_all(none);
}

/// Ticket #170 (`AP6`): with the approved reviews and the private art pack
/// on this machine, Ryu's and Ken's throws, thrown sets and get-ups load.
/// Tickets #171 and #172 (`AP7`, `AP8`): so does Sagat's whole set, both
/// halves approved. Tickets #173 and #174 (`AP9`, `AP10`): so does Retsu's.
/// Tickets #175 and #176 (`AP11`, `AP12`): so does Geki's. Tickets #177
/// and #178 (`AP13`, `AP14`): so does Joe's. Tickets #179 and #180
/// (`AP15`, `AP16`): so does Mike's. Tickets #181 and #182 (`AP17`,
/// `AP18`): so does Lee's. Tickets #183 and #184 (`AP19`, `AP20`): so
/// does Gen's. Tickets #185 and #186 (`AP21`, `AP22`): so does Birdie's.
/// Tickets #187 and #188 (`AP23`, `AP24`): so does Eagle's. Tickets #189
/// and #190 (`AP25`, `AP26`): so does Adon's, the last of the twelve.
/// Skips without the ROMs or the pack.
#[test]
fn the_local_pack_loads_every_approved_fighter() {
    let pack = Path::new(ROMS).parent().unwrap().join("sf1-port-art");
    if !Path::new(ROMS).is_dir() || !pack.is_dir() {
        eprintln!("skipping: local ROM set or art pack not present");
        return;
    }
    let rom = load_assets(ROMS, MANIFEST).unwrap();
    let (mut log, path) = scratch_log("local-pack");
    let art = load_from(&rom, &RECIPES, &REVIEWS, find_pack(&[pack]), &mut log).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    for who in ["ryu", "ken", "sagat", "retsu", "geki", "joe", "mike", "lee", "gen", "birdie", "eagle", "adon"] {
        assert!(!text.contains(&format!("art: {who} keeps his ROM art")), "{text}");
    }
    let sets: [(&str, &[&str]); 12] = [
        ("ryu", &["get-up", "throw", "thrown"]),
        ("ken", &["get-up", "throw", "thrown"]),
        (
            "sagat",
            &[
                "crouch", "crouching block", "crouching hit", "crouching punch", "crouching kick", "sweep", "throw", "thrown", "get-up",
                "jump", "jumping punch", "jumping diagonal punch", "jumping kick", "jumping heavy kick", "jumping diagonal kick", "low tiger shot",
            ],
        ),
        (
            "retsu",
            &[
                "crouching punch", "crouching kick", "throw", "thrown", "get-up",
                "jumping punch", "jumping diagonal punch", "jumping kick", "jumping heavy kick", "kiai shot",
            ],
        ),
        (
            "geki",
            &[
                "standing block", "crouching block", "jab", "light kick", "crouching kick", "sweep", "throw", "thrown", "get-up",
                "jumping punch", "jumping kick", "jumping heavy kick", "jumping diagonal kick", "teleport strike",
            ],
        ),
        (
            "joe",
            &[
                "crouch", "crouching block", "crouching hit", "crouching kick", "sweep", "throw", "thrown", "get-up",
                "jumping punch", "jumping diagonal punch", "jumping kick", "jumping heavy kick", "rolling sobat", "dynamite straight",
            ],
        ),
        (
            "mike",
            &[
                "crouching block", "crouching hit", "body hook", "overhand right", "low body hook", "crouching uppercut", "throw", "thrown",
                "get-up", "jumping punch", "jumping diagonal punch", "jumping low jab", "jumping hook", "jumping hammer", "bison rush",
                "iron upper",
            ],
        ),
        (
            "lee",
            &[
                "crouch", "crouching block", "crouching hit", "crouching punch", "crouching kick", "throw", "thrown", "get-up",
                "jumping punch", "jumping diagonal punch", "jumping kick", "zesshouohken", "double kick",
            ],
        ),
        (
            "gen",
            &[
                "crouch", "crouching block", "crouching hit", "crouching punch", "crouching kick", "low kick", "throw", "thrown", "get-up",
                "jumping punch", "jumping diagonal punch", "jumping kick", "gekiro", "hyakurenko",
            ],
        ),
        (
            "birdie",
            &[
                "crouch", "crouching block", "crouching hit", "crouching punch", "crouching kick", "sweep", "high kick", "throw", "thrown",
                "get-up", "jumping punch", "jumping kick", "jumping heavy kick", "jumping diagonal kick", "bull horn",
            ],
        ),
        (
            "eagle",
            &[
                "crouch", "crouching block", "crouching hit", "crouching stick sweep", "crouching stick uppercut", "rising stick strike",
                "two-stick smash", "throw", "thrown", "get-up", "jumping high strike", "jumping stick jab", "jumping stick poke",
                "jumping stick swing", "jumping two-stick smash", "manchester black", "st andrews green",
            ],
        ),
        (
            "adon",
            &[
                "crouch", "crouching block", "crouching hit", "crouching punch", "crouching kick", "sweep", "throw", "thrown", "get-up",
                "jumping punch", "jumping diagonal punch", "jumping kick", "jumping heavy kick", "jaguar kick", "rising jaguar",
            ],
        ),
    ];
    for (who, moves) in sets {
        let catalogue = art.catalogue(who).unwrap_or_else(|| panic!("{who} loads: {text}"));
        for m in moves {
            assert!(catalogue.moves.iter().any(|mv| mv.name == *m), "{who} {m}: {text}");
        }
    }
    println!("{text}");
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn a_damaged_pack_is_logged_and_its_fighters_keep_their_rom_art() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let rom = load_assets(ROMS, MANIFEST).unwrap();
    let (mut log, path) = scratch_log("pack");
    // Ryu's set approved in his committed review, but no usable pack.
    let damaged = "x/sf1-port-art: manifest.json: expected value";
    let art = load_from(&rom, &[recipe("ryu")], &REVIEWS, Err(damaged.to_owned()), &mut log).unwrap();
    assert!(art.catalogue("ryu").is_none(), "he has generated poses");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains(&format!("art: ryu keeps his ROM art: his generated pose throw_turn needs the art pack, and {damaged}")),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
