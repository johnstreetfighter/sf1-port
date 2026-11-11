//! Ticket #193 (`ST1`): the ten 1987 fight stages as stage records in the
//! extended background bank, read from the ROM's tables.
//!
//! The expected values come from this file's own reading of the ROM's four
//! countries' tables, not from the builder: every starting country lists
//! the opponents in its own rotation, so an opponent is found by searching
//! each country's scroll table, and every table that has it must agree on
//! its background, its tune and its country's palettes. Local ROMs only;
//! each test skips without them.

use std::path::Path;

use sf_video::background_bank::BackgroundBank;
use sf_video::busts::FIGHTERS;
use sf_video::romset::{load_assets, Assets};
use sf_video::stage::pack_records;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const TEST_STAGE: &str = include_str!("../../../art/stages/test-stage.json");

/// The tables, one longword per starting country (0 Japan, 1 China, 2 USA,
/// 3 England) each pointing to ten words (fg scroll, bg scroll, tune) or
/// five longwords (palettes).
const FG_SCROLLS: usize = 0x437ee;
const BG_SCROLLS: usize = 0x437fe;
const MUSIC: usize = 0x42358;
const FG_PALETTES: usize = 0x436e8;
const BG_PALETTES: usize = 0x43778;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

fn word(m: &[u8], at: usize) -> usize {
    u16::from_be_bytes([m[at], m[at + 1]]) as usize
}

fn long(m: &[u8], at: usize) -> usize {
    word(m, at) << 16 | word(m, at + 2)
}

/// Opponent index `i` of start `country` in `tables`.
fn entry(m: &[u8], tables: usize, country: usize, i: usize) -> usize {
    word(m, long(m, tables + 4 * country) + 2 * i)
}

/// The palette of stage `stage` of start `country` in `tables`.
fn palette(m: &[u8], tables: usize, country: usize, stage: usize) -> usize {
    long(m, long(m, tables + 4 * country) + 4 * stage)
}

#[test]
fn the_ten_1987_stages_are_listed_in_the_ladders_order_after_the_test_stage() {
    let Some(a) = assets() else { return };
    let mut bank = pack_records(&a, &[TEST_STAGE]).expect("the test stage packs");
    assert_eq!(bank.add_rom_stages(&a), Ok(10));
    let owners = ["Retsu", "Geki", "Joe", "Mike", "Lee", "Gen", "Birdie", "Eagle", "Adon", "Sagat"];
    assert_eq!(&FIGHTERS[2..], &owners);
    let places = ["Japan", "Japan", "USA", "USA", "China", "China", "England", "England", "Thailand", "Thailand"];
    let listed = bank.listed_stages();
    assert_eq!(listed.len(), 10, "the test stage is never listed");
    for (k, l) in listed.iter().enumerate() {
        assert_eq!((l.owner.as_str(), l.place.as_str()), (owners[k], places[k]));
        assert_eq!(bank.stage_index(&owners[k].to_ascii_lowercase()), Some(l.index));
        assert!(bank.stage(l.index).is_some());
    }
    let names: Vec<&str> = bank.stage_names().collect();
    assert_eq!(names[0], "test-stage");
    assert_eq!(names.len(), 11);
    assert_eq!(bank.stage_index("test-stage"), Some(0), "the test stage keeps its place");
}

#[test]
fn each_stage_is_the_rom_tables_answer_for_its_opponent_from_every_start() {
    let Some(a) = assets() else { return };
    let m = &a.maincpu;
    let mut bank = BackgroundBank::new();
    bank.add_rom_stages(&a).unwrap();
    for (k, listing) in bank.listed_stages().iter().enumerate() {
        let view = bank.stage(listing.index).unwrap();
        let what = format!("{} (opponent {k})", listing.owner);
        // The scroll words are the opponent's on the ROM's own page.
        assert_eq!((view.fg.page, view.bg.page), (0, 0), "{what}");
        let mut found = Vec::new();
        for country in 0..4 {
            let at = (0..10).filter(|&i| entry(m, FG_SCROLLS, country, i) == view.fg.scroll as usize).collect::<Vec<_>>();
            assert_eq!(at.len(), 1, "{what}: country {country} lists the foreground word once");
            let i = at[0];
            assert_eq!(entry(m, BG_SCROLLS, country, i), view.bg.scroll as usize, "{what}: country {country}'s background word");
            assert_eq!(entry(m, MUSIC, country, i), view.theme as usize, "{what}: country {country}'s tune");
            // Palettes are by country and stage only: the stage the opponent
            // is on, in that start's ladder.
            assert_eq!(palette(m, FG_PALETTES, country, i / 2) as u32, view.fg.palette, "{what}: country {country}'s foreground palette");
            assert_eq!(palette(m, BG_PALETTES, country, i / 2) as u32, view.bg.palette, "{what}: country {country}'s background palette");
            found.push((country, i));
        }
        // The start in the opponent's own country brings him first (stage
        // 0, the fight's index); Thailand is never a start, so Adon and
        // Sagat are last from every start.
        if k < 8 {
            assert!(found.iter().any(|&(_, i)| i == k % 2), "{what}: some start opens on him: {found:?}");
        } else {
            assert!(found.iter().all(|&(_, i)| i == k), "{what}: last from every start: {found:?}");
        }
    }
    // The ten stages are ten different backdrops, tunes and countries.
    let views: Vec<_> = bank.listed_stages().iter().map(|l| bank.stage(l.index).unwrap()).collect();
    for i in 0..10 {
        for j in i + 1..10 {
            assert_ne!(views[i].fg.scroll, views[j].fg.scroll);
            assert_ne!(views[i].bg.scroll, views[j].bg.scroll);
            assert_ne!(views[i].theme, views[j].theme);
        }
    }
    let palettes = |f: fn(&sf_video::background_bank::StageView) -> u32| {
        let mut p: Vec<u32> = views.iter().map(f).collect();
        p.dedup();
        p
    };
    assert_eq!(palettes(|v| v.fg.palette).len(), 5, "a country's two stages share a palette");
    assert_eq!(palettes(|v| v.bg.palette).len(), 5);
}

#[test]
fn the_bank_of_1987_stages_alone_is_not_empty_and_adds_no_pages_or_tiles() {
    let Some(a) = assets() else { return };
    let mut bank = BackgroundBank::new();
    assert!(bank.is_empty());
    bank.add_rom_stages(&a).unwrap();
    assert!(!bank.is_empty());
    assert_eq!(bank.pages(), 0);
    assert_eq!(a.with_background_bank(&bank).background_pages(), 1, "the ROM's page alone");
}

/// Each owner's stage written out by hand, not read through the builder's
/// code: foreground scroll, background scroll, stage tune, foreground
/// palette, background palette. The scroll words are Japan's tables in
/// `docs/reference/background-format.md` section 3 (Retsu, Geki, Joe,
/// Mike, Lee, Gen, Birdie, Eagle, Adon, Sagat); `both-chips.md` names `0x30`
/// as Thailand's Sagat loop; the tunes and palettes are the ROM's tables as
/// dumped (`0x28`..`0x31` in the opponent order of the tunes' countries;
/// palettes `0x1a468`.. and `0x1b068`.. by country, Japan to Thailand).
/// Retsu and Geki stand in Japan, Joe and Mike in the USA, Lee and Gen in
/// China, Birdie and Eagle in England, Adon and Sagat in Thailand, so a
/// label a country off would fail here.
const PINNED: [(&str, u16, u16, u16, u32, u32); 10] = [
    ("retsu", 0x1c80, 0x1800, 0x28, 0x1a468, 0x1b068),
    ("geki", 0x1f80, 0x2180, 0x29, 0x1a468, 0x1b068),
    ("joe", 0x2400, 0x2b00, 0x2a, 0x1a668, 0x1b268),
    ("mike", 0x2700, 0x3480, 0x2b, 0x1a668, 0x1b268),
    ("lee", 0x2b80, 0x3e00, 0x2e, 0x1a868, 0x1b468),
    ("gen", 0x2e80, 0x4780, 0x2f, 0x1a868, 0x1b468),
    ("birdie", 0x3300, 0x5100, 0x2c, 0x1aa68, 0x1b668),
    ("eagle", 0x3600, 0x5a80, 0x2d, 0x1aa68, 0x1b668),
    ("adon", 0x3a80, 0x6400, 0x30, 0x1ac68, 0x1b868),
    ("sagat", 0x3d80, 0x6d80, 0x31, 0x1ac68, 0x1b868),
];

#[test]
fn each_owner_is_pinned_to_the_values_the_reference_docs_give() {
    let Some(a) = assets() else { return };
    let mut bank = BackgroundBank::new();
    bank.add_rom_stages(&a).unwrap();
    for (name, fg, bg, tune, fg_palette, bg_palette) in PINNED {
        let view = bank.stage(bank.stage_index(name).unwrap_or_else(|| panic!("{name}"))).unwrap();
        assert_eq!(
            (view.fg.scroll, view.bg.scroll, view.theme, view.fg.palette, view.bg.palette),
            (fg, bg, tune, fg_palette, bg_palette),
            "{name}"
        );
    }
}

#[test]
fn a_stage_name_that_clashes_is_an_error_and_leaves_the_bank_as_it_was() {
    let Some(a) = assets() else { return };
    // The stages added twice.
    let mut bank = BackgroundBank::new();
    bank.add_rom_stages(&a).unwrap();
    let before = bank.clone();
    let err = bank.add_rom_stages(&a).expect_err("a second time clashes");
    assert!(err.contains("retsu"), "{err}");
    assert_eq!(bank, before);
    // A record named after an owner.
    let mut bank = pack_records(&a, &[TEST_STAGE]).unwrap();
    let view = bank.stage(0).unwrap();
    bank.add_stage("sagat", view);
    let before = bank.clone();
    assert!(bank.add_rom_stages(&a).expect_err("a clash").contains("sagat"));
    assert_eq!(bank, before);
    assert!(bank.listed_stages().is_empty());
}

#[test]
fn the_ladder_names_the_stage_of_every_step_from_every_start() {
    let Some(a) = assets() else { return };
    let m = &a.maincpu;
    let mut bank = BackgroundBank::new();
    bank.add_rom_stages(&a).unwrap();
    for country in 0..4 {
        let mut seen = Vec::new();
        for step in 0..10 {
            let name = sf_video::rom_stages::ladder_stage_name(m, country, step).unwrap();
            let index = bank.stage_index(&name).unwrap_or_else(|| panic!("{name} is a stage of the bank"));
            // The stage's tune is the one the country's table has at that step.
            assert_eq!(bank.stage(index).unwrap().theme as usize, entry(m, MUSIC, country, step), "country {country} step {step}: {name}");
            seen.push(name);
        }
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), 10, "country {country} meets each opponent once");
    }
    // Japan's ladder is the stages' own order, so Geki is its second step.
    assert_eq!(sf_video::rom_stages::ladder_stage_name(m, 0, 1).unwrap(), "geki");
    assert!(sf_video::rom_stages::ladder_stage_name(m, 0, 10).is_err());
    assert!(sf_video::rom_stages::ladder_stage_name(m, 4, 0).is_err());
}
