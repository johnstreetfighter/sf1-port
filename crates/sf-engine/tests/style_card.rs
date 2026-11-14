//! Anniversary Edition Style card (#6).

use std::path::Path;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::era::{EraFamily, Loadout, Revision, StylePick};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::menu::MenuItem;
use sf_engine::front::select::SelectScreen;
use sf_engine::front::settings::{Row, Settings, SettingsScreen};
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, PlayerInputs, Transition};
use sf_engine::roster::{MenuInput, TournamentFighter};
use sf_video::compositor::{darken, fallback_text, ink, RomFont};
use sf_video::render::{Frame, HEIGHT, WIDTH};
use sf_video::romset::{load_assets, Assets};

fn assert_style(pick: StylePick, family: EraFamily, revision: Revision, loadout: Loadout) {
    assert_eq!(
        (pick.family, pick.revision, pick.loadout),
        (family, revision, loadout)
    );
}

#[test]
fn sides_edit_and_lock_independently_with_flagship_defaults_and_single_option_rows_skipped() {
    use MenuInput::{Back, Confirm, Down, Left, Right, Start, Up};
    use TournamentFighter::{Ken, Ryu};

    let mut select = SelectScreen::local(&MatchConfig::default(), &[]);
    assert_eq!(select.apply_side(0, Confirm), Some(Transition::StyleOpened));
    assert_eq!(
        select.apply_side(0, Down),
        None,
        "SF1+ has one revision and loadout"
    );
    assert_eq!(select.apply_side(1, Start), Some(Transition::StyleOpened));
    assert_eq!(select.apply_side(1, Left), Some(Transition::CursorMoved));
    assert_eq!(select.apply_side(1, Left), Some(Transition::CursorMoved));
    assert_eq!(
        select.apply_side(1, Up),
        None,
        "SF6 has one revision and loadout"
    );
    assert_eq!(select.apply_side(1, Confirm), Some(Transition::Picked(Ken)));
    assert_style(
        select.locked_style(1).unwrap(),
        EraFamily::Sf6,
        Revision::Sf6,
        Loadout::Standard,
    );
    assert_eq!(select.apply_side(0, Back), Some(Transition::Unpicked));
    assert_eq!(select.locked(1), Some(Ken));
    assert_eq!(select.fighter(0), Ryu);

    assert_eq!(select.apply_side(0, Confirm), Some(Transition::StyleOpened));
    assert_eq!(select.apply_side(0, Right), Some(Transition::CursorMoved)); // SF2 ST
    assert_eq!(select.apply_side(0, Down), Some(Transition::CursorMoved)); // REV
    assert_eq!(select.apply_side(0, Right), Some(Transition::CursorMoved)); // WW
    assert_eq!(select.apply_side(0, Down), Some(Transition::CursorMoved)); // skips LOAD
    assert_eq!(select.apply_side(0, Right), Some(Transition::CursorMoved)); // SFZ3
    assert_eq!(select.apply_side(0, Up), Some(Transition::CursorMoved)); // LOAD
    assert_eq!(select.apply_side(0, Right), Some(Transition::CursorMoved)); // V-ISM
    assert_eq!(
        select.apply_side(0, Start),
        Some(Transition::Fighters(Ryu, Ken))
    );
    assert_style(
        select.locked_style(0).unwrap(),
        EraFamily::SfZ,
        Revision::Sfz3,
        Loadout::VIsm,
    );
}

#[test]
fn only_the_overhaul_and_balanced_custom_offer_the_card() {
    for preset in RulesetPreset::ALL {
        let config = MatchConfig::from_preset(preset);
        let mut screen = SelectScreen::local(&config, &[]);
        let locked = screen.fighter(0);
        let expected = if config.era_select_enabled() {
            Transition::StyleOpened
        } else {
            Transition::Picked(locked)
        };
        assert_eq!(
            screen.apply_side(0, MenuInput::Confirm),
            Some(expected),
            "{preset:?}"
        );
    }
    let balanced =
        MatchConfig::from_preset(RulesetPreset::ClassicCompetitive).with_balanced_roster(true);
    assert_eq!(balanced.preset, RulesetPreset::Custom);
    assert!(balanced.era_select_enabled());
    let mut arcade = SelectScreen::arcade(&balanced, &[]);
    assert_eq!(
        arcade.apply(MenuInput::Confirm),
        Some(Transition::StyleOpened)
    );
    assert_eq!(
        arcade.apply(MenuInput::Start),
        Some(Transition::Fighter(TournamentFighter::Ryu))
    );
    assert_eq!(arcade.locked_style(0), Some(StylePick::default()));
}

#[test]
fn what_if_setting_controls_the_available_era_and_revision_lists() {
    let mut settings = SettingsScreen::open(MatchConfig::default(), Settings::default());
    settings.apply(MenuInput::TabRight);
    for _ in 0..3 {
        settings.apply(MenuInput::Down);
    }
    assert_eq!(settings.row(), Row::WhatIfStyles);
    assert_eq!(settings.value(Row::WhatIfStyles), "ON");
    assert_eq!(
        settings.apply(MenuInput::Confirm),
        Some(Transition::ValueChanged)
    );
    assert_eq!(settings.value(Row::WhatIfStyles), "OFF");

    for (config, expected) in [
        (MatchConfig::default(), (EraFamily::Sf2, Revision::Hf)),
        (
            *settings.config(),
            (EraFamily::Sf1987, Revision::Arcade1987),
        ),
    ] {
        let mut select = SelectScreen::local(&config, &TournamentFighter::ALL);
        for _ in 0..4 {
            select.apply_side(0, MenuInput::Right);
        }
        assert_eq!(select.fighter(0), TournamentFighter::Joe);
        select.apply_side(0, MenuInput::Confirm);
        select.apply_side(0, MenuInput::Right);
        select.apply_side(0, MenuInput::Down);
        if config.roster.what_if_styles {
            select.apply_side(0, MenuInput::Right); // ST wraps to the only other revision, HF.
        }
        select.apply_side(0, MenuInput::Confirm);
        assert_style(
            select.locked_style(0).unwrap(),
            expected.0,
            expected.1,
            Loadout::Standard,
        );
    }
}

fn assets() -> Option<Assets> {
    let roms = "../../tools/reference/local/roms/sf";
    if !Path::new(roms).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(
        load_assets(roms, "../../tools/reference/rom-manifest.json")
            .expect("local ROM set assembles"),
    )
}

fn tap(session: &mut GameSession, keys: impl FnOnce(&mut FrameInputs)) -> FrameOutput {
    let mut input = FrameInputs::default();
    keys(&mut input);
    let out = session.step(input);
    session.step(FrameInputs::default());
    out
}

fn choose(session: &mut GameSession, item: MenuItem) -> FrameOutput {
    for _ in 0..8 {
        if session.menu_item() == Some(item) {
            return tap(session, |i| i.menu.confirm = true);
        }
        tap(session, |i| i.p1.directions[2] = true);
    }
    panic!("{item:?} missing from {}", session.screen());
}

fn open_menu(session: &mut GameSession) {
    for _ in 0..200 {
        session.step(FrameInputs::default());
    }
    assert_eq!(tap(session, |i| i.menu.start = true).screen, "main menu");
}

fn choose_country(session: &mut GameSession) {
    for _ in 0..60 {
        if session.screen() == "country select" {
            assert_eq!(tap(session, |i| i.p1.buttons[0] = true).screen, "fight");
            return;
        }
        session.step(FrameInputs::default());
    }
    panic!("country select did not open");
}

fn assert_text(frame: &Frame, x: usize, y: usize, text: &str, color: u32) {
    let mut expected = vec![0; WIDTH * HEIGHT];
    fallback_text(&mut expected, x, y, text, color, 1);
    for (actual, glyph) in frame.argb.iter().zip(expected) {
        if glyph != 0 {
            assert_eq!(*actual, glyph, "missing text {text:?}");
        }
    }
}

fn confirm_both(session: &mut GameSession) -> FrameOutput {
    tap(session, |i| {
        let mut player = PlayerInputs::default();
        player.buttons[0] = true;
        i.p1 = player;
        i.p2 = player;
    })
}

#[test]
fn local_cards_darken_the_portraits_update_the_badge_and_remember_each_era() {
    let Some(rom) = assets() else { return };
    let font = RomFont::new(&rom.gfx4, &rom.maincpu);
    let mut session = GameSession::new(assets().expect("local ROM set"), MatchConfig::default());
    open_menu(&mut session);
    choose(&mut session, MenuItem::MultiPlayer);
    let grid = choose(&mut session, MenuItem::Local);
    let p1 = tap(&mut session, |i| i.p1.buttons[0] = true);
    assert_eq!(p1.transition, Some(Transition::StyleOpened));
    assert_eq!(p1.screen, "select");
    assert_eq!(p1.frame.argb[35 * WIDTH + 51], 0xffca_d8ff);
    assert_eq!(p1.frame.argb[35 * WIDTH + 146], 0xffca_d8ff);

    // The card dims just P1's bust. The pixels between the spinners and
    // captions still show the portrait at one-quarter brightness.
    let mut dimmed = grid.frame.argb.clone();
    darken(&mut dimmed, 50, 30, 98, 98);
    let mut changed = false;
    for y in 30..128 {
        if matches!(y, 30..=34 | 36..=38 | 86..=96 | 102..=114 | 120..=127) {
            let strip = y * WIDTH + 50..y * WIDTH + 148;
            assert_eq!(p1.frame.argb[strip.clone()], dimmed[strip.clone()]);
            changed |= p1.frame.argb[strip.clone()] != grid.frame.argb[strip];
        }
        let p2_panel = y * WIDTH + 234..y * WIDTH + 336;
        assert_eq!(p1.frame.argb[p2_panel.clone()], grid.frame.argb[p2_panel]);
    }
    assert!(changed);

    tap(&mut session, |i| i.p1.directions[0] = true);
    tap(&mut session, |i| i.p1.directions[0] = true);
    let sf3 = tap(&mut session, |i| i.p1.directions[0] = true);
    assert_text(
        &sf3.frame,
        53,
        97,
        "SHINKUU HADOKEN (2 STK)",
        font.pen(ink::GOLD_A, 0),
    );
    assert_text(
        &sf3.frame,
        166,
        60,
        "SF3 3S · SA I",
        font.pen(ink::YELLOW, 0),
    );
    tap(&mut session, |i| i.p2.buttons[0] = true);
    tap(&mut session, |i| i.p2.directions[0] = true); // SF2 ST
    tap(&mut session, |i| i.p2.directions[3] = true); // LOAD
    let old = tap(&mut session, |i| i.p2.directions[0] = true); // Old
    assert_text(
        &old.frame,
        241,
        97,
        "O.KEN (NO SUPER GAUGE)",
        font.pen(ink::GOLD_A, 0),
    );
    assert_text(
        &old.frame,
        168,
        116,
        "SF2 ST · OLD",
        font.pen(ink::P2_BLUE, 0),
    );
    tap(&mut session, |i| i.p2.directions[2] = true); // ERA
    tap(&mut session, |i| i.p2.directions[1] = true); // SF1+
    tap(&mut session, |i| i.p2.directions[1] = true); // SF1
    assert_eq!(
        confirm_both(&mut session).transition,
        Some(Transition::Fighters(
            TournamentFighter::Ryu,
            TournamentFighter::Ken
        ))
    );
    let fight = session.fight().expect("real local fight");
    assert_style(
        fight.fighter_style(0).unwrap(),
        EraFamily::Sf3,
        Revision::Sf3ThreeS,
        Loadout::Art1,
    );
    assert_style(
        fight.fighter_style(1).unwrap(),
        EraFamily::Sf1987,
        Revision::Arcade1987,
        Loadout::Standard,
    );

    choose_country(&mut session);
    for _ in 0..3000 {
        if session.fight_machine().map(flow_position) == Some(0x3eafa) {
            break;
        }
        session.step(FrameInputs::default());
    }
    assert_eq!(
        tap(&mut session, |i| i.menu.start = true).transition,
        Some(Transition::Paused)
    );
    for _ in 0..5 {
        tap(&mut session, |i| i.p1.directions[2] = true);
    }
    assert_eq!(
        tap(&mut session, |i| i.menu.confirm = true).transition,
        Some(Transition::Choose(MenuItem::QuitToMenu))
    );
    choose(&mut session, MenuItem::MultiPlayer);
    choose(&mut session, MenuItem::Local);
    assert_eq!(
        confirm_both(&mut session).transition,
        Some(Transition::StyleOpened)
    );
    confirm_both(&mut session);
    let next = session.fight().expect("next local fight");
    assert_style(
        next.fighter_style(0).unwrap(),
        EraFamily::Sf3,
        Revision::Sf3ThreeS,
        Loadout::Art1,
    );
    assert_style(
        next.fighter_style(1).unwrap(),
        EraFamily::Sf1987,
        Revision::Arcade1987,
        Loadout::Standard,
    );
}

#[test]
fn arcade_resolves_the_exact_revision_or_fallback_on_each_native_ladder_slot() {
    for what_if in [true, false] {
        let Some(rom) = assets() else { return };
        let mut config = MatchConfig::default();
        config.roster.what_if_styles = what_if;
        let mut session = GameSession::new(rom, config);
        open_menu(&mut session);
        choose(&mut session, MenuItem::SinglePlayer);
        choose(&mut session, MenuItem::Arcade);
        tap(&mut session, |i| i.menu.confirm = true);
        tap(&mut session, |i| i.p1.directions[0] = true); // SF2 ST
        tap(&mut session, |i| i.p1.directions[2] = true); // REV
        tap(&mut session, |i| i.p1.directions[0] = true); // WW
        tap(&mut session, |i| i.menu.start = true);
        choose_country(&mut session);

        let mut fight = session.fight().expect("real Arcade fight").clone();
        assert_style(
            fight.fighter_style(0).unwrap(),
            EraFamily::Sf2,
            Revision::Ww,
            Loadout::Standard,
        );
        let mut snapshot = fight.save_netplay_snapshot(0);
        // Visit all ten positions of the country table selected by the
        // native drive. The counters identify the opponent at round start.
        let mut visited = Vec::new();
        for step in 0..10 {
            snapshot.machine.work[0x4003] = step / 2;
            snapshot.machine.work[0x4004] = step % 2;
            fight.restore_netplay_snapshot(&snapshot);
            let opponent = fight.machine().arcade_opponent().expect("ladder opponent");
            assert!(!visited.contains(&opponent));
            visited.push(opponent);
            let cpu = fight.fighter_style(1).expect("ladder style");
            if opponent == TournamentFighter::Sagat {
                assert_style(cpu, EraFamily::Sf2, Revision::Ww, Loadout::Standard);
            } else if what_if {
                let loadout = if step >= 7 && step % 2 == 1 {
                    Loadout::Old
                } else {
                    Loadout::Standard
                };
                assert_style(cpu, EraFamily::Sf2, Revision::St, loadout);
            } else {
                assert_eq!(cpu, StylePick::default());
            }
        }
    }
}
