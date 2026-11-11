//! Every boss's 1987 player table in the extended data bank (ADR-0014,
//! spec #246) against the move registry: the bank is built from every
//! recipe in `art/recipes` with its review, and for each fighter who gets a
//! table, each special slot his table has is listed in the registry once,
//! on the same command (tracker and button) as his table puts it, and each
//! registry entry his table brings is one of his slots. The list MOVES
//! shows and the coach's Try are written from the registry, so this keeps
//! them true to what the fight does. Local ROMs and the local art pack
//! only; skips without them.

use std::path::Path;

use sf_engine::config::RulesetPreset;
use sf_engine::data_bank::{in_bank, special_slot, DataBank, SF2_SPECIAL_SLOT, SF2_SUPER_SLOT, SPECIAL_SLOTS};
use sf_engine::roster::TournamentFighter;
use sf_engine::moves::{move_list, ButtonClass, Recognizer};
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::romset::load_assets;

const ROMS: &str = "../../tools/reference/local/roms/sf";
const PACK: &str = "../../tools/reference/local/roms/sf1-port-art";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";

#[test]
fn every_bank_fighters_special_slots_are_the_registrys_entries_on_the_same_commands() {
    if !Path::new(ROMS).is_dir() || !Path::new(PACK).is_dir() {
        eprintln!("skipping: local ROM set or art pack not present");
        return;
    }
    let a = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
    let read = |dir: &str, name: &str| std::fs::read_to_string(Path::new("../../art").join(dir).join(name)).ok();
    let mut recipes = Vec::new();
    let mut reviews = Vec::new();
    let mut names: Vec<String> = std::fs::read_dir("../../art/recipes")
        .expect("the recipes")
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".json"))
        .collect();
    names.sort();
    for name in &names {
        recipes.push(read("recipes", name).unwrap());
        reviews.push(read("review", name).unwrap_or_else(|| "{}".to_owned()));
    }
    let recipes: Vec<&str> = recipes.iter().map(String::as_str).collect();
    let reviews: Vec<&str> = reviews.iter().map(String::as_str).collect();
    let art = load_art_from(&a, &ArtSources { recipes: &recipes, reviews: Some(&reviews), pack: Ok(&pack) }).expect("the art loads");
    let bank = DataBank::build(&a.maincpu, &art.catalogues);
    println!("{:#?}", bank.notes());
    assert!(!bank.is_empty(), "some boss has a table: {:?}", bank.notes());
    for &fighter in bank.fighters() {
        let table = bank.table(fighter).unwrap();
        let rb = |addr: u32| bank.rb(addr);
        let present: Vec<u8> = SPECIAL_SLOTS.into_iter()
            .filter(|&slot| bank.move_header(fighter, slot).is_some_and(|header| rb(header + 8) != 0))
            .collect();
        for preset in [RulesetPreset::ClassicCompetitive, RulesetPreset::DefinitiveOverhaul, RulesetPreset::PureFootsies1987Plus] {
            let entries: Vec<_> = move_list(fighter, preset).into_iter().filter(|e| e.available.player_table).collect();
            for &slot in &present {
                let listed: Vec<_> = entries.iter().filter(|e| e.produces == slot).collect();
                assert_eq!(listed.len(), 1, "{fighter:?} {preset:?}: special {slot:#04x} is listed once: {listed:?}");
                let s = special_slot(rb, table, slot);
                let Recognizer::Tracker(tracker) = listed[0].recognizer else {
                    panic!("{fighter:?}: legacy special {slot:#04x} uses a 1987 tracker");
                };
                assert_eq!(
                    (tracker.offset(), listed[0].input.button == ButtonClass::Kick),
                    (s.tracker, s.kick),
                    "{fighter:?}: {} is on his table's command",
                    listed[0].name
                );
            }
            for e in &entries {
                assert!(present.contains(&e.produces), "{fighter:?}: {} brings a special his table has", e.name);
            }
        }
    }
}

#[test]
fn shoto_sf2_slots_use_rom_poses_without_replacing_the_rom_binding() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    use TournamentFighter::{Ken, Ryu};
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let bank = DataBank::build(&assets.maincpu, &[]);
    for fighter in [Ryu, Ken] {
        assert_eq!(bank.table(fighter), None, "the legacy fighter remains ROM-bound");
        assert!(bank.throw_entry(fighter).is_some(), "ROM throw fallback remains available");
    }
    for (fighter, slot) in [(Ryu, SF2_SPECIAL_SLOT), (Ryu, SF2_SUPER_SLOT), (Ken, SF2_SUPER_SLOT)] {
        let header = bank.move_header(fighter, slot).expect("the SF2 move has an authored header");
        assert_eq!(bank.rb(header + 8), 1);
        assert_eq!(bank.rb(header + 0xa), slot);
        let stream = u32::from_be_bytes(std::array::from_fn(|i| bank.rb(header + 0x2c + i as u32)));
        assert!(in_bank(stream), "{fighter:?} move {slot:#04x} starts a native bank stream");
    }
    assert_eq!(bank.move_header(Ken, SF2_SPECIAL_SLOT), None);
}
