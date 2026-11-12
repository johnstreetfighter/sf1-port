use super::*;
use crate::arcade::ending_epilogue;
use crate::data_bank::DataBank;
use crate::fighter::WORK_BASE;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::{ArtPack, PACK_FOLDER};
use sf_video::ending_screens::ending_key;
use sf_video::romset::load_assets;
use std::path::Path;

fn work(addr: u32) -> usize {
    (addr - WORK_BASE) as usize
}

fn advance_to(fight: &mut Fight, slot: u32, position: u32, budget: u32) -> bool {
    let mut saw_stand_in = false;
    for _ in 0..budget {
        let positions = fight.machine.task_positions();
        saw_stand_in |=
            positions.contains(&(1, 0x44ae8)) && fight.machine.state.bg_scroll == 0x0900;
        if positions.contains(&(slot, position)) {
            assert!(
                fight.machine.unrecovered().is_empty(),
                "{:?}",
                fight.machine.unrecovered()
            );
            return saw_stand_in;
        }
        fight.machine.step();
    }
    panic!(
        "{:?} did not reach {position:#x}: {:?}",
        fight.setup.config.roster.p1,
        fight.machine.task_positions()
    );
}

fn ending_text(video: &[u8]) -> String {
    (0..13)
        .flat_map(|row| {
            (0..33)
                .map(move |col| {
                    let at = 0x822 + row * 0x80 + col * 2;
                    match u16::from_be_bytes(video[at..at + 2].try_into().unwrap()) & 0x3ff {
                        glyph @ 0x0a..=0x23 => char::from(b'A' + (glyph - 0x0a) as u8),
                        0x27 => '.',
                        0x29 => '\'',
                        0x2a => ',',
                        _ => ' ',
                    }
                })
                .chain([' '])
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn assert_epilogue(fight: &Fight, backgrounds: &BackgroundBank, fighter: TournamentFighter) {
    let state = &fight.machine.state;
    let actual_text = ending_text(&state.videoram);
    if let Some(text) = ending_epilogue(fighter) {
        let view = backgrounds.view(&ending_key(fighter.name())).unwrap();
        assert_eq!(
            (state.bg_page, state.bg_scroll),
            (view.page, view.scroll),
            "{fighter:?} ending view"
        );
        assert_eq!(actual_text, text, "{fighter:?} epilogue");
    } else {
        let scroll = if fighter == TournamentFighter::Ryu {
            0x7700
        } else {
            0x7880
        };
        assert_eq!(
            (state.bg_page, state.bg_scroll),
            (0, scroll),
            "{fighter:?} ending view"
        );
        assert!(
            actual_text.starts_with("YOU HAVE EARNED THE DISTINCTION OF KING OF THE HILL"),
            "{actual_text}"
        );
    }
}

#[test]
fn all_twelve_arcade_fights_take_the_thailand_win_through_name_entry() {
    let repo = Path::new("../..");
    let roms = repo.join("tools/reference/local/roms/sf");
    let pack_path = repo.join("assets").join(PACK_FOLDER);
    if !roms.is_dir() || !pack_path.is_dir() {
        eprintln!("skipping: local ROMs or art pack not present");
        return;
    }
    let rom = load_assets(roms, repo.join("tools/reference/rom-manifest.json")).unwrap();
    let pack = ArtPack::open(&pack_path).unwrap();
    let read_art = |folder: &str| -> Vec<String> {
        TournamentFighter::ALL
            .into_iter()
            .map(|fighter| {
                let name = fighter.name().to_ascii_lowercase();
                std::fs::read_to_string(repo.join("art").join(folder).join(format!("{name}.json")))
                    .unwrap()
            })
            .collect()
    };
    let recipes = read_art("recipes");
    let reviews = read_art("review");
    let recipe_refs = recipes.iter().map(String::as_str).collect::<Vec<_>>();
    let review_refs = reviews.iter().map(String::as_str).collect::<Vec<_>>();
    let art = load_art_from(
        &rom,
        &ArtSources {
            recipes: &recipe_refs,
            reviews: Some(&review_refs),
            pack: Ok(&pack),
        },
    )
    .unwrap();
    let data = Arc::new(DataBank::build(&rom.maincpu, &art.catalogues));
    assert_eq!(data.fighters().len(), 10, "{:?}", data.notes());
    let mut backgrounds = BackgroundBank::new();
    backgrounds.add_vs_screens(&rom).unwrap();
    backgrounds.add_ending_screens(&rom).unwrap();
    let backgrounds = Arc::new(backgrounds);
    let assets = rom
        .with_background_bank(&backgrounds)
        .with_sprite_bank(&art.bank);

    for fighter in TournamentFighter::ALL {
        let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
        config.roster.p1 = fighter;
        config.p2_fair_cpu = true;
        let setup = FightSetup {
            kind: FightKind::Arcade,
            players: Players::One,
            config,
            dsw1: FightDips::DSW1,
            dsw2: FightDips::default().dsw2(),
        };
        let mut fight = Fight::start_banked(
            &assets.maincpu,
            setup,
            Some(0),
            Some(backgrounds.clone()),
            Some(data.clone()),
        )
        .unwrap_or_else(|event| panic!("{fighter:?}: {event:?}"));
        assert_eq!(flow_position(&fight.machine), 0x3e59c);
        fight.black = 0;
        let mut snapshot = fight.machine.save_snapshot();
        // Enter Thailand's second fight through the native stage intro.
        snapshot.work[work(0xffc003)] = 4;
        snapshot.work[work(0xffc004)] = 1;
        fight.machine.restore_snapshot(&snapshot);
        fight.step(
            &assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
        advance_to(&mut fight, 1, FIGHT_LOOP, 1800);
        assert_eq!(fight.player_fighter(0), fighter);

        let mut snapshot = fight.machine.save_snapshot();
        // One round is already won. Force the last round's CPU KO and a
        // qualifying score. Native tasks handle everything after the KO.
        snapshot.work[work(P1_STRUCT + 8)] = 1;
        snapshot.work[work(P1_STRUCT + 0x22)..work(P1_STRUCT + 0x29)]
            .copy_from_slice(&[9, 0, 0, 0, 0, 0, 0]);
        snapshot.state.objectram[0xa08] = 8;
        fight.machine.restore_snapshot(&snapshot);
        fight.step(
            &assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
        advance_to(&mut fight, 2, 0x401e6, 1800);
        let defeated = if fighter == TournamentFighter::Sagat {
            0x0900
        } else {
            0x7100
        };
        assert_eq!(
            fight.machine.state.bg_scroll, defeated,
            "{fighter:?} win card"
        );

        let saw_stand_in = advance_to(&mut fight, 1, 0x44b78, 3500);
        assert_eq!(saw_stand_in, !fighter.is_shoto(), "{fighter:?} montage");
        assert_epilogue(&fight, &backgrounds, fighter);

        fight.step(
            &assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
        assert!(!fight.is_over());
        advance_to(&mut fight, 1, 0x44c04, 2400);
        advance_to(&mut fight, 1, 0x42552, 300);
        assert_eq!(
            fight.machine.state.fg_scroll, 0x4080,
            "{fighter:?} name entry"
        );
        assert_eq!(
            &fight.machine.work_ram()[work(0xff8e6a)..work(0xff8e6e)],
            b"111\0"
        );
        fight.step(
            &assets,
            PlayerInputs::default(),
            PlayerInputs::default(),
            false,
        );
        assert!(!fight.is_over());
    }
}

#[test]
fn tour_finale_uses_the_selected_shoto_or_boss_ending() {
    use crate::config::WorldTourRules;
    use crate::roster::PlayMode;
    use crate::world_tour::native::node;

    let roms = "../../tools/reference/local/roms/sf";
    if !Path::new(roms).is_dir() {
        eprintln!("skipping: local ROMs not present");
        return;
    }
    let rom = load_assets(roms, "../../tools/reference/rom-manifest.json").unwrap();
    let data = Arc::new(DataBank::build(&rom.maincpu, &[]));
    let mut backgrounds = BackgroundBank::new();
    backgrounds.add_ending_screens(&rom).unwrap();
    let backgrounds = Arc::new(backgrounds);

    for fighter in [
        TournamentFighter::Ryu,
        TournamentFighter::Ken,
        TournamentFighter::Retsu,
        TournamentFighter::Sagat,
    ] {
        let mut config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
        config.roster.mode = PlayMode::Versus;
        config.roster.p1 = fighter;
        config.world_tour = Some(WorldTourRules {
            mission: node("th-2-era").unwrap().mission,
            bonuses: None,
            starting_hp: None,
        });
        let setup = FightSetup {
            kind: FightKind::WorldTour,
            players: Players::Two,
            config,
            dsw1: FightDips::DSW1,
            dsw2: FightDips::default().dsw2(),
        };
        let mut fight = Fight::start_banked(
            &rom.maincpu,
            setup,
            Some(0),
            Some(backgrounds.clone()),
            Some(data.clone()),
        )
        .unwrap_or_else(|event| panic!("{fighter:?}: {event:?}"));
        fight.start_world_tour_ending();
        assert!(!advance_to(&mut fight, 1, 0x44b78, 1000));
        assert_epilogue(&fight, &backgrounds, fighter);
    }
}
