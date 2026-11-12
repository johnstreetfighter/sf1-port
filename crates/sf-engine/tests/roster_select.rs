//! Ticket #98 (O7): the twelve-fighter roster.
//!
//! Character select, ban lists, and the four-country flag are pure. Binding
//! each fighter into a live fight uses the versus and CPU checkpoints.

use std::path::PathBuf;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fight::FightMachine;
use sf_engine::fighter::{OBJ_BASE, P1_STRUCT, P2_STRUCT};
use sf_engine::roster::{
    BanFormat, CharacterSelectScreen, MenuInput, PlayMode, RosterSettings, StartingCountry,
    TournamentFighter,
};
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::compositor::{select_cell_origin, SELECT_GOLD, SELECT_YELLOW};
use sf_video::render::WIDTH;
use sf_video::romset::{load_assets, load_program_rom};

const RECIPES: [&str; 11] = [
    include_str!("../../../art/recipes/ryu.json"),
    include_str!("../../../art/recipes/retsu.json"),
    include_str!("../../../art/recipes/geki.json"),
    include_str!("../../../art/recipes/joe.json"),
    include_str!("../../../art/recipes/mike.json"),
    include_str!("../../../art/recipes/lee.json"),
    include_str!("../../../art/recipes/gen.json"),
    include_str!("../../../art/recipes/birdie.json"),
    include_str!("../../../art/recipes/eagle.json"),
    include_str!("../../../art/recipes/adon.json"),
    include_str!("../../../art/recipes/sagat.json"),
];
const REVIEWS: [&str; 11] = [
    include_str!("../../../art/review/ryu.json"),
    include_str!("../../../art/review/retsu.json"),
    include_str!("../../../art/review/geki.json"),
    include_str!("../../../art/review/joe.json"),
    include_str!("../../../art/review/mike.json"),
    include_str!("../../../art/review/lee.json"),
    include_str!("../../../art/review/gen.json"),
    include_str!("../../../art/review/birdie.json"),
    include_str!("../../../art/review/eagle.json"),
    include_str!("../../../art/review/adon.json"),
    include_str!("../../../art/review/sagat.json"),
];

fn data_bank() -> std::sync::Arc<sf_engine::data_bank::DataBank> {
    let root = repo_root();
    let assets = load_assets(
        root.join("tools/reference/local/roms/sf"),
        root.join("tools/reference/rom-manifest.json"),
    )
    .expect("local ROM set");
    let pack = ArtPack::open(&root.join("assets/sf1-port-art")).expect("local art pack");
    let art = load_art_from(&assets, &ArtSources { recipes: &RECIPES, reviews: Some(&REVIEWS), pack: Ok(&pack) }).expect("art loads");
    std::sync::Arc::new(sf_engine::data_bank::DataBank::build(&assets.maincpu, &art.catalogues))
}

fn expected_table(f: TournamentFighter) -> u32 {
    if f.is_shoto() {
        f.move_table()
    } else {
        let slot = TournamentFighter::ALL.iter().position(|&x| x == f).unwrap() as u32;
        0x10_0000 + slot * 0x1_0000
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn load_rom() -> Vec<u8> {
    let root = repo_root();
    load_program_rom(
        &root.join("tools/reference/local/roms/sf"),
        &root.join("tools/reference/rom-manifest.json"),
    )
    .expect("local ROM set")
}

fn versus_with(cfg: MatchConfig) -> FightMachine {
    let root = repo_root();
    FightMachine::from_checkpoint_2450_with_program(
        &root.join("tools/reference/local/captures/move-run-a"),
        load_rom(),
    )
    .expect("checkpoint 2450")
    .with_match_config(cfg)
    .with_data_bank(data_bank())
}

fn arcade_with(cfg: MatchConfig) -> FightMachine {
    let root = repo_root();
    FightMachine::from_checkpoint_2389(
        &root.join("tools/reference/local/captures/cpu-01-run-a"),
        load_rom(),
    )
    .expect("checkpoint 2389")
    .with_match_config(cfg)
}

fn definitive_cast(p1: TournamentFighter, p2: TournamentFighter) -> MatchConfig {
    let mut cfg = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    cfg.roster.p1 = p1;
    cfg.roster.p2 = p2;
    cfg
}

fn entity_index(m: &FightMachine, ps: u32) -> usize {
    let w = m.work_ram();
    let off = (ps - 0xff8000) as usize + 0x10;
    let entity = u16::from_be_bytes([w[off], w[off + 1]]);
    (OBJ_BASE + entity as u32 - 0xffe000) as usize
}

fn move_id(m: &FightMachine, ps: u32) -> u8 {
    m.state.objectram[entity_index(m, ps) + 0x0a]
}

fn state_byte(m: &FightMachine, ps: u32) -> u8 {
    m.state.objectram[entity_index(m, ps) + 0x09]
}

fn pos_x(m: &FightMachine, ps: u32) -> u16 {
    let b = entity_index(m, ps) + 0x1e;
    let o = &m.state.objectram;
    u16::from_be_bytes([o[b], o[b + 1]])
}

fn move_table(m: &FightMachine, ps: u32) -> u32 {
    let w = m.work_ram();
    let off = (ps - 0xff8000) as usize + 0x12;
    u32::from_be_bytes([w[off], w[off + 1], w[off + 2], w[off + 3]])
}

fn health(m: &FightMachine, ps: u32) -> u16 {
    let w = m.work_ram();
    let off = (ps - 0xff8000) as usize;
    (w[off + 7] as u16) * 4 + (w[off + 6] as u16)
}

fn knocked_out(m: &FightMachine, ps: u32) -> bool {
    m.state.objectram[entity_index(m, ps) + 8] == 8
}

fn wins(m: &FightMachine, ps: u32) -> u8 {
    m.work_ram()[(ps - 0xff8000) as usize + 8]
}

fn cursor_name(screen: &CharacterSelectScreen) -> Option<String> {
    screen.rows().into_iter().find_map(|row| {
        let rest = row.strip_prefix("> ")?;
        let name = rest.split_whitespace().nth(1)?;
        Some(name.to_string())
    })
}

fn select_fighter(screen: &mut CharacterSelectScreen, fighter: TournamentFighter) {
    for _ in 0..16 {
        if cursor_name(screen).as_deref() == Some(fighter.name()) {
            screen.apply(MenuInput::Confirm);
            return;
        }
        screen.apply(MenuInput::Right);
    }
    panic!("could not reach {}", fighter.name());
}

fn finish_select(screen: &mut CharacterSelectScreen, mode: PlayMode) {
    if mode != PlayMode::Arcade {
        screen.apply(MenuInput::Confirm);
    }
    for _ in 0..8 {
        match screen.phase_name() {
            "Country" | "Era family" | "Revision" | "Costume" => screen.apply(MenuInput::Confirm),
            _ => break,
        }
    }
}

#[test]
fn select_offers_all_twelve_in_arcade_versus_and_training() {
    for mode in [PlayMode::Arcade, PlayMode::Versus, PlayMode::Training] {
        let mut settings = RosterSettings::for_preset(RulesetPreset::DefinitiveOverhaul);
        settings.mode = mode;
        settings.ban = BanFormat::Unrestricted;
        for fighter in TournamentFighter::ALL {
            let mut screen = CharacterSelectScreen::open(settings, true);
            assert_eq!(screen.rows().len() >= 12 + 3, true);
            select_fighter(&mut screen, fighter);
            finish_select(&mut screen, mode);
            assert!(
                screen.commit(&mut settings),
                "{mode:?} {fighter:?} {}",
                screen.phase_name()
            );
            assert_eq!(settings.p1, fighter, "{mode:?}");
            let roster_rows = screen
                .rows()
                .into_iter()
                .filter(|row| {
                    row.starts_with("> ") || row.starts_with("x ") || row.starts_with("  ")
                })
                .count();
            assert_eq!(roster_rows, 12, "{mode:?}");
        }
    }
}

#[test]
fn four_country_cursor_covers_japan_usa_china_england() {
    let mut settings = RosterSettings::for_preset(RulesetPreset::ClassicCompetitive);
    settings.mode = PlayMode::Arcade;
    let mut screen = CharacterSelectScreen::open(settings, true);
    screen.apply(MenuInput::Confirm);
    assert_eq!(screen.phase_name(), "Country");
    let mut seen = Vec::new();
    for _ in 0..4 {
        let row = screen
            .rows()
            .into_iter()
            .find(|row| row.starts_with("COUNTRY CURSOR"))
            .expect("country cursor");
        seen.push(row);
        screen.apply(MenuInput::Right);
    }
    let joined = seen.join(" | ");
    for name in ["Japan", "USA", "China", "England"] {
        assert!(joined.contains(name), "{joined}");
    }
}

#[test]
fn ban_lists_match_the_three_sf1pp_formats() {
    for fighter in TournamentFighter::ALL {
        let standard = BanFormat::StandardTournament.allows(fighter, 0, true);
        assert_eq!(standard, fighter != TournamentFighter::Sagat, "{fighter:?}");
        let low = BanFormat::LowTierNoShoto.allows(fighter, 0, true);
        assert_eq!(low, !fighter.is_shoto(), "{fighter:?}");
        assert!(BanFormat::Unrestricted.allows(fighter, 0, false));
    }
    assert!(!BanFormat::StandardTournament.allows(TournamentFighter::Ryu, 0, false));
    assert!(BanFormat::StandardTournament.allows(TournamentFighter::Ryu, 1, false));
    assert!(BanFormat::StandardTournament.allows(TournamentFighter::Ryu, 0, true));

    // Classic Competitive starts with no ban list; the format is a choice.
    let mut settings = RosterSettings::for_preset(RulesetPreset::ClassicCompetitive);
    assert_eq!(settings.ban, BanFormat::Unrestricted);
    settings.ban = BanFormat::StandardTournament;
    let screen = CharacterSelectScreen::open(settings, true);
    let sagat = screen
        .rows()
        .into_iter()
        .find(|row| row.contains("Sagat"))
        .unwrap();
    assert!(sagat.contains('x') && sagat.contains("BANNED"), "{sagat}");
    assert_ne!(cursor_name(&screen).as_deref(), Some("Sagat"));
}

#[test]
fn direct_arcade_ken_needs_no_versus_join() {
    let mut machine = arcade_with(definitive_cast(
        TournamentFighter::Ken,
        TournamentFighter::Ryu,
    ));
    assert_eq!(machine.work_ram()[0x4002], 2, "1P arcade stays mode 2");
    let before = pos_x(&machine, P1_STRUCT);
    for _ in 0..8 {
        machine.set_p1_direction(true, false, false, false);
        machine.step();
    }
    assert_eq!(
        move_table(&machine, P1_STRUCT),
        TournamentFighter::Ken.move_table()
    );
    assert!(
        pos_x(&machine, P1_STRUCT) != before || move_id(&machine, P1_STRUCT) != 1,
        "Ken walked or left stance on the 1P side"
    );
    assert!(
        machine.unrecovered().is_empty(),
        "{:08x?}",
        machine.unrecovered()
    );
}

#[test]
fn each_fighter_can_move_and_attack_on_both_sides() {
    for fighter in TournamentFighter::ALL {
        for side in [1u8, 2u8] {
            let (p1, p2, ps) = if side == 1 {
                (fighter, TournamentFighter::Ken, P1_STRUCT)
            } else {
                (TournamentFighter::Ryu, fighter, P2_STRUCT)
            };
            let mut machine = versus_with(definitive_cast(p1, p2));
            let before = pos_x(&machine, ps);
            for _ in 0..12 {
                if side == 1 {
                    machine.set_p1_direction(true, false, false, false);
                } else {
                    machine.set_p2_direction(false, true, false, false);
                }
                machine.step();
            }
            let moved = pos_x(&machine, ps) != before || move_id(&machine, ps) != 1;
            assert!(moved, "{fighter:?} side {side} did not move");
            assert_eq!(move_table(&machine, ps), expected_table(fighter));

            let mut machine = versus_with(definitive_cast(p1, p2));
            for _ in 0..6 {
                if side == 1 {
                    machine.set_p1_buttons([true, false, false, false, false, false]);
                } else {
                    machine.set_p2_buttons([true, false, false, false, false, false]);
                }
                machine.step();
            }
            let attacked = state_byte(&machine, ps) & 0x40 != 0 || move_id(&machine, ps) > 1;
            assert!(attacked, "{fighter:?} side {side} did not attack");
            assert!(
                machine.unrecovered().is_empty(),
                "{fighter:?} side {side} {:08x?}",
                machine.unrecovered()
            );
        }
    }
}

fn place_in_range(machine: &mut FightMachine, mine: u32, theirs: u32, side: u8) {
    machine.with_bus(|bus| {
        bus.bind_roster_tables();
        let (me, other) = bus.player_entities(mine);
        let _ = theirs;
        let center = bus.center_x(other);
        let at = if side == 1 {
            center.wrapping_sub(44)
        } else {
            center.wrapping_add(44)
        };
        bus.start_move_at(me, mine, 1, at);
    });
}

fn tap_attack(machine: &mut FightMachine, side: u8) {
    for frame in 0..8 {
        let buttons = [false, false, frame < 2, false, false, false];
        if side == 1 {
            machine.set_p1_direction(false, false, false, false);
            machine.set_p1_buttons(buttons);
        } else {
            machine.set_p2_direction(false, false, false, false);
            machine.set_p2_buttons(buttons);
        }
        machine.step();
    }
}

#[test]
fn each_fighter_can_finish_a_round_on_both_sides() {
    for fighter in TournamentFighter::ALL {
        for side in [1u8, 2u8] {
            let (p1, p2, mine, theirs) = if side == 1 {
                (fighter, TournamentFighter::Ken, P1_STRUCT, P2_STRUCT)
            } else {
                (TournamentFighter::Ryu, fighter, P2_STRUCT, P1_STRUCT)
            };
            let mut machine = versus_with(definitive_cast(p1, p2));
            machine.set_work_byte(theirs + 7, 0);
            machine.set_work_byte(theirs + 6, 1);
            let wins_before = wins(&machine, mine);
            assert_eq!(health(&machine, theirs), 1, "{fighter:?} side {side}");
            place_in_range(&mut machine, mine, theirs, side);
            for _ in 0..4 {
                machine.step();
            }
            let mut finished = false;
            for _ in 0..30 {
                tap_attack(&mut machine, side);
                if knocked_out(&machine, theirs) || wins(&machine, mine) > wins_before {
                    finished = true;
                    break;
                }
            }
            assert!(
                finished,
                "{fighter:?} side {side} left the opponent at {} HP, move {:02x}, x {:04x}/{:04x}",
                health(&machine, theirs),
                move_id(&machine, mine),
                pos_x(&machine, mine),
                pos_x(&machine, theirs)
            );
            let mut guard = 0;
            while wins(&machine, mine) == wins_before && guard < 240 {
                machine.step();
                guard += 1;
            }
            assert!(
                wins(&machine, mine) > wins_before,
                "{fighter:?} side {side} KO did not record a round win"
            );
            assert!(
                machine.unrecovered().is_empty(),
                "{fighter:?} side {side} {:08x?}",
                machine.unrecovered()
            );
        }
    }
}

#[test]
fn overhaul_opens_four_countries_and_faithful_keeps_the_dip() {
    let capture = repo_root().join("tools/reference/local/captures/credits-01-run-a");
    if !capture.join("checkpoint-2140.json").exists() && !capture.exists() {
        eprintln!("skipping: credits-01-run-a is not present");
        return;
    }
    let mut faithful =
        FightMachine::from_fight_checkpoint(&capture, 2140, load_rom()).expect("checkpoint 2140");
    faithful.set_match_config(MatchConfig::faithful());
    faithful.set_work_byte(0xffc005, 1);
    faithful.set_cabinet_inputs(false, false, true, false, false);
    faithful.step();
    assert!(
        faithful.task_positions().contains(&(1, 0x3e0e6)),
        "faithful 1987 with a two-country DIP stays on 0x3e0e6, got {:?}",
        faithful.task_positions()
    );
    assert!(faithful.unrecovered().is_empty());

    let mut overhaul =
        FightMachine::from_fight_checkpoint(&capture, 2140, load_rom()).expect("checkpoint 2140");
    overhaul.set_match_config(MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul));
    overhaul.set_work_byte(0xffc005, 1);
    overhaul.set_cabinet_inputs(false, false, true, false, false);
    overhaul.step();
    assert!(
        overhaul.task_positions().contains(&(1, 0x3e252)),
        "overhaul opens four countries even when 0xffc005 is nonzero, got {:?}",
        overhaul.task_positions()
    );
    assert!(overhaul.unrecovered().is_empty());
}

#[test]
fn faithful_roster_leaves_the_original_tables_alone() {
    let cfg = MatchConfig::faithful();
    assert!(!cfg.roster.full_cast);
    assert!(!cfg.roster.four_country);
    let mut machine = versus_with(cfg.clone());
    let p1 = move_table(&machine, P1_STRUCT);
    let p2 = move_table(&machine, P2_STRUCT);
    for _ in 0..4 {
        machine.set_p1_direction(true, false, false, false);
        machine.step();
    }
    assert_eq!(move_table(&machine, P1_STRUCT), p1);
    assert_eq!(move_table(&machine, P2_STRUCT), p2);

    let mut cfg = cfg;
    cfg.roster.p1 = TournamentFighter::Sagat;
    let mut machine = versus_with(cfg);
    for _ in 0..4 {
        machine.set_p1_buttons([true, false, false, false, false, false]);
        machine.step();
    }
    assert_eq!(
        move_table(&machine, P1_STRUCT),
        TournamentFighter::Ryu.move_table()
    );
}

#[test]
fn roster_round_trips_in_toml_without_retargeting_the_preset() {
    let mut cfg = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
    cfg.roster.p1 = TournamentFighter::Adon;
    cfg.roster.country = StartingCountry::China;
    cfg.roster.ban = BanFormat::LowTierNoShoto;
    let parsed = MatchConfig::from_toml(&cfg.to_toml()).unwrap();
    assert_eq!(parsed.preset, RulesetPreset::DefinitiveOverhaul);
    assert_eq!(parsed.roster.p1, TournamentFighter::Adon);
    assert_eq!(parsed.roster.country, StartingCountry::China);
    assert_eq!(parsed.roster.ban, BanFormat::LowTierNoShoto);
    assert!(parsed.roster.full_cast);
    assert!(parsed.roster.four_country);
}

#[test]
fn select_type_and_overhaul_hud_leave_the_faithful_frame_alone() {
    use sf_video::compositor::{present, OverhaulScene, PresentMode};
    use sf_video::render::{render_frame, HEIGHT, WIDTH};
    use sf_video::romset::load_assets;

    let settings = RosterSettings::for_preset(RulesetPreset::DefinitiveOverhaul);
    let screen = CharacterSelectScreen::open(settings, true);
    let lines = screen.rows();
    let hardware = sf_video::render::Frame {
        argb: vec![0xff_11_22_33; WIDTH * HEIGHT],
    };
    let typed = present(
        PresentMode::Overhaul,
        &hardware,
        &OverhaulScene {
            select_lines: &lines,
            ..OverhaulScene::EMPTY
        },
    );
    assert_eq!(typed.argb.len(), WIDTH * HEIGHT);
    assert!(
        lines[0].starts_with('S'),
        "the select screen's first line is the title"
    );
    assert_eq!(typed.argb[0], 0xff_e8_d8_b0);
    let skipped = present(
        PresentMode::Faithful,
        &hardware,
        &OverhaulScene {
            select_lines: &lines,
            ..OverhaulScene::EMPTY
        },
    );
    assert_eq!(skipped.argb, hardware.argb);

    let root = repo_root();
    let assets = load_assets(
        root.join("tools/reference/local/roms/sf"),
        root.join("tools/reference/rom-manifest.json"),
    )
    .expect("local ROM set");
    let mut faithful = versus_with(MatchConfig::faithful());
    faithful.set_overlay_enabled(false);
    let base = render_frame(&assets, &faithful.state.scene());
    assert_eq!(faithful.render_with_overlay(&assets).argb, base.argb);

    let witness = 0xff_39_c1_07;
    let mut sprites = Vec::new();
    for i in 0..79 {
        let color = 0xff_00_00_00 | ((i as u32 + 1) * 0x0103);
        sprites.push(sf_video::compositor::IndexedSprite::solid(
            (i % 24) * 16,
            8 + (i / 24) * 16,
            color,
        ));
    }
    sprites.push(sf_video::compositor::IndexedSprite::solid(
        300, 180, witness,
    ));
    assert_eq!(sprites.len(), 80);

    faithful.set_overhaul_sprites(sprites.clone());
    faithful.set_select_lines(lines.clone());
    assert_eq!(
        faithful.render_with_overlay(&assets).argb,
        base.argb,
        "faithful play ignores extra sprites and select type"
    );

    let mut overhaul = versus_with(MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul));
    overhaul.set_overlay_enabled(false);
    overhaul.set_overhaul_sprites(sprites);
    overhaul.set_select_lines(lines.clone());
    let hardware = render_frame(&assets, &overhaul.state.scene());
    let framed = overhaul.render_with_overlay(&assets);
    assert_eq!(framed.argb.len(), WIDTH * HEIGHT);
    assert_eq!(
        framed.argb[0], 0xff_e8_d8_b0,
        "select type is inked on the frame"
    );
    assert_eq!(
        framed.argb[180 * WIDTH + 300],
        witness,
        "the 80th sprite is on the overhaul frame"
    );
    let corners = (HEIGHT - 24) * WIDTH;
    assert_eq!(
        &framed.argb[corners..],
        &hardware.argb[corners..],
        "the 1987 fighters carry no era state, so no era gauges are drawn"
    );

    overhaul.set_overlay_enabled(true);
    let with_training = overhaul.render_with_overlay(&assets);
    assert_ne!(with_training.argb, framed.argb);
    assert_eq!(with_training.argb.len(), WIDTH * HEIGHT);
}

#[test]
fn portrait_grid_moves_down_a_column_and_paints_the_cursor() {
    let settings = RosterSettings::for_preset(RulesetPreset::DefinitiveOverhaul);
    let mut screen = CharacterSelectScreen::open(settings, true);
    assert_eq!(screen.portrait_slot(), 0);
    assert_eq!(screen.portrait_caption(), "Ryu");
    let ryu = screen.portrait_frame();
    let (x, y) = select_cell_origin(0);
    assert_eq!(ryu.argb[y * WIDTH + x], SELECT_YELLOW);
    screen.apply(MenuInput::Down);
    assert_eq!(screen.portrait_slot(), 3);
    assert_eq!(screen.portrait_caption(), "Geki");
    let geki = screen.portrait_frame();
    let (gx, gy) = select_cell_origin(3);
    assert_eq!(geki.argb[gy * WIDTH + gx], SELECT_YELLOW);
    assert_eq!(geki.argb[y * WIDTH + x], SELECT_GOLD);
    screen.apply(MenuInput::Right);
    assert_eq!(screen.portrait_slot(), 4);
}
