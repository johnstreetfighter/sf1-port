//! The real cabinet input loop consumes translated pad frames.

#[path = "support/alpha.rs"]
mod alpha;

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::fair_cpu::FIGHT_LOOP;
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{Fight, FightDips, FightKind, FightSetup};
use sf_engine::front::PlayerInputs;
use sf_engine::roster::PlayMode;
use sf_engine::session_io::ControlScheme;
use sf_video::romset::{load_assets, Assets};
use std::path::Path;
use std::sync::OnceLock;

fn assets() -> Option<&'static Assets> {
    static ASSETS: OnceLock<Option<Assets>> = OnceLock::new();
    ASSETS
        .get_or_init(|| {
            let roms = Path::new("../../tools/reference/local/roms/sf");
            if !roms.exists() {
                eprintln!("skipping: local ROM set is unavailable");
                return None;
            }
            Some(
                load_assets(roms, "../../tools/reference/rom-manifest.json")
                    .expect("assemble the ROM set"),
            )
        })
        .as_ref()
}

fn bout(assets: &Assets, preset: RulesetPreset, scheme: ControlScheme, penalty: bool) -> Fight {
    let mut config = MatchConfig::from_preset(preset);
    config.roster.mode = PlayMode::Versus;
    config.p1_scheme = scheme;
    config.modern_control_damage_penalty = penalty;
    let mut fight = Fight::start(
        &assets.maincpu,
        FightSetup {
            kind: FightKind::Local,
            players: Players::Two,
            config,
            dsw1: FightDips::DSW1,
            dsw2: FightDips::default().dsw2(),
        },
        Some(0),
        None,
    )
    .unwrap();
    for _ in 0..1_500 {
        tick(&mut fight, assets, PlayerInputs::default());
        if flow_position(fight.machine()) == FIGHT_LOOP
            && fight.fight_read().fighters[0].move_id == 1
        {
            return fight;
        }
    }
    panic!("fight did not reach a playable stance");
}

fn tick(fight: &mut Fight, assets: &Assets, p1: PlayerInputs) {
    let out = fight.step(assets, p1, PlayerInputs::default(), false);
    assert!(out.event.is_none(), "{:?}", out.event);
}

fn pad(bits: u8, button: Option<usize>) -> PlayerInputs {
    PlayerInputs {
        directions: std::array::from_fn(|i| bits & (1 << i) != 0),
        buttons: std::array::from_fn(|i| button == Some(i)),
    }
}

#[test]
fn pneumatic_resolves_punch_and_kick_pressure_once() {
    let Some(assets) = assets() else { return };
    let ready = bout(
        assets,
        RulesetPreset::DefinitiveOverhaul,
        ControlScheme::Pneumatic,
        false,
    );
    for (button, moves) in [(0, [0x0d, 0x0f, 0x10]), (3, [0x14, 0x15, 0x16])] {
        for (duration, level) in [(1, 0), (3, 1), (5, 2)] {
            let mut fight = ready.clone();
            for frame in 1..=duration {
                tick(&mut fight, assets, pad(0, Some(button)));
                if frame < 5 {
                    assert_eq!(fight.fight_read().fighters[0].move_id, 1);
                }
            }
            if duration < 5 {
                tick(&mut fight, assets, pad(0, None));
            }
            assert_eq!(
                fight.fight_read().fighters[0].move_id,
                moves[level],
                "{duration} frame pad {button}"
            );
        }
    }

    let mut fight = ready.clone();
    for _ in 0..3 {
        tick(&mut fight, assets, pad(0, Some(0)));
    }
    let bookmark = fight.save_netplay_snapshot(3);
    tick(&mut fight, assets, pad(0, None));
    let expected = fight.state_checksum();
    fight.restore_netplay_snapshot(&bookmark);
    tick(&mut fight, assets, pad(0, None));
    assert_eq!(
        fight.state_checksum(),
        expected,
        "pressure counters survive rollback"
    );
}

fn health(fight: &Fight) -> u16 {
    let fighter = fight.machine().era_fighters()[1];
    if fighter.max_sub_hp != 0 {
        fighter.sub_hp
    } else {
        fight.fight_read().fighters[1].health
    }
}

fn landed_damage(
    mut fight: Fight,
    assets: &Assets,
    expected_slot: u8,
    command: &[PlayerInputs],
) -> u16 {
    let before = health(&fight);
    for input in command {
        tick(&mut fight, assets, *input);
    }
    assert_eq!(fight.fight_read().fighters[0].move_id, expected_slot);
    for _ in 0..160 {
        tick(&mut fight, assets, pad(0, None));
        if health(&fight) < before {
            return before - health(&fight);
        }
    }
    panic!("move {expected_slot:#x} never hit");
}

#[test]
fn modern_uses_native_shoryuken_and_scales_only_shortcut_damage() {
    let Some(assets) = assets() else { return };
    let mut base = bout(
        assets,
        RulesetPreset::DefinitiveOverhaul,
        ControlScheme::Modern,
        true,
    );
    let mut open = base.clone();
    let mut rules = open.save_netplay_snapshot(0);
    rules.setup.config.modern_control_damage_penalty = false;
    rules.machine.match_config.modern_control_damage_penalty = false;
    open.restore_netplay_snapshot(&rules);
    let short = [pad(0, Some(3))];
    let motion = [pad(4, None), pad(5, None), pad(1, Some(2))];
    let penalty = landed_damage(base.clone(), assets, 0x1a, &short);
    let unpenalized = landed_damage(open.clone(), assets, 0x1a, &short);
    let manual = landed_damage(base.clone(), assets, 0x1a, &motion);
    assert_eq!(
        manual, unpenalized,
        "manual motion uses the original damage"
    );
    assert_eq!(
        penalty,
        unpenalized * 4 / 5,
        "shortcut applies 80 percent once"
    );
    for fight in [&mut base, &mut open] {
        for _ in 0..120 {
            let positions = fight.machine().diagnostic_telemetry();
            if positions.p1.center_x.abs_diff(positions.p2.center_x) <= 25 {
                break;
            }
            tick(fight, assets, pad(1, None));
        }
    }
    let short = [pad(1, Some(3))];
    let motion = [pad(1, None), pad(4, None), pad(5, Some(2))];
    let penalty = landed_damage(base.clone(), assets, 0x19, &short);
    let unpenalized = landed_damage(open, assets, 0x19, &short);
    let manual = landed_damage(base, assets, 0x19, &motion);
    assert_eq!(manual, unpenalized);
    assert_eq!(penalty, unpenalized * 4 / 5);
}

#[test]
fn modern_base_specials_work_under_classic_competitive_rules() {
    let Some(assets) = assets() else { return };
    let base = bout(
        assets,
        RulesetPreset::ClassicCompetitive,
        ControlScheme::Modern,
        true,
    );
    let shortcut = landed_damage(base.clone(), assets, 0x1a, &[pad(0, Some(3))]);
    let motion = [pad(4, None), pad(5, None), pad(1, Some(2))];
    let manual = landed_damage(base, assets, 0x1a, &motion);
    assert!(shortcut > 0 && shortcut < manual);
}

#[test]
fn faithful_uses_the_same_cabinet_inputs_even_with_other_saved_scheme_choices() {
    let Some(assets) = assets() else { return };
    let mut classic = bout(
        assets,
        RulesetPreset::Faithful1987,
        ControlScheme::Classic,
        false,
    );
    let mut configured = bout(
        assets,
        RulesetPreset::Faithful1987,
        ControlScheme::Modern,
        false,
    );
    for input in [
        pad(0, Some(3)),
        pad(0, None),
        pad(1, Some(2)),
        pad(4, None),
        pad(5, None),
    ] {
        tick(&mut classic, assets, input);
        tick(&mut configured, assets, input);
        assert_eq!(
            classic.machine().work_image(),
            configured.machine().work_image()
        );
        assert_eq!(classic.state_checksum(), configured.state_checksum());
    }
}

#[test]
fn modern_finisher_uses_the_native_sf6_meter_and_move() {
    use sf_engine::era::{GaugeMode, Loadout, Revision};

    let Some(session) =
        alpha::local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard)
    else {
        return;
    };
    let assets = session.assets();
    let mut fight = alpha::local_fight(&session, Revision::Sf1Plus, Loadout::Standard)
        .fight()
        .unwrap()
        .clone();
    let mut rules = fight.save_netplay_snapshot(0);
    rules.setup.config.p1_scheme = ControlScheme::Modern;
    rules.machine.match_config.p1_scheme = ControlScheme::Modern;
    fight.restore_netplay_snapshot(&rules);
    let input = pad(5, Some(3));
    tick(&mut fight, assets, input);
    assert_ne!(fight.fight_read().fighters[0].move_id, 0x2b);
    rules.machine.toggle_state.era.fighters[0].super_meter = 144;
    fight.restore_netplay_snapshot(&rules);
    tick(&mut fight, assets, input);
    assert_eq!(fight.fight_read().fighters[0].move_id, 0x2b);
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
}

#[test]
fn modern_assist_keeps_a_followup_pressed_during_startup() {
    let Some(assets) = assets() else { return };
    let mut fight = bout(
        assets,
        RulesetPreset::DefinitiveOverhaul,
        ControlScheme::Modern,
        false,
    );
    let mut light = pad(0, Some(0));
    light.buttons[4] = true;
    let assist = pad(0, Some(4));

    tick(&mut fight, assets, light);
    assert_eq!(fight.fight_read().fighters[0].move_id, 0x0d);
    tick(&mut fight, assets, assist);
    tick(&mut fight, assets, light);
    assert_eq!(fight.fight_read().fighters[0].move_id, 0x0d);
    let mut followed = false;
    for _ in 0..36 {
        tick(&mut fight, assets, assist);
        followed |= fight.fight_read().fighters[0].move_id == 0x14;
    }
    assert!(
        followed,
        "the buffered press starts standing LK when actionable"
    );
}
