//! SF1+ defensive moves on the native fight path (ADR-0015).

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod alpha;

use std::path::Path;
use std::sync::{Arc, OnceLock};

use alpha::{pad, DOWN, LEFT, LP, MANIFEST, RIGHT, ROMS};
use sf_engine::combo::throw::{RANGE, THROWN_MOVE, THROW_MOVE};
use sf_engine::config::MatchConfig;
use sf_engine::data_bank::DataBank;
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{Fight, FightDips, FightKind, FightSetup};
use sf_engine::front::PlayerInputs;
use sf_engine::roster::{PlayMode, TournamentFighter};
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::romset::{load_assets, Assets};

const MP: usize = 1;
const HP: usize = 2;
const LK: usize = 3;
const MK: usize = 4;
const ART_PACK: &str = "../../assets/sf1-port-art";
const FIGHT_LOOP: u32 = 0x3eafa;

fn assets_and_bank() -> Option<&'static (Assets, Arc<DataBank>)> {
    static FIXTURE: OnceLock<(Assets, Arc<DataBank>)> = OnceLock::new();
    if !Path::new(ROMS).is_dir() || !Path::new(ART_PACK).is_dir() {
        eprintln!("skipping: local ROMs or art pack are absent");
        return None;
    }
    Some(FIXTURE.get_or_init(|| {
        let assets = load_assets(ROMS, MANIFEST).expect("local ROMs assemble");
        let read = |directory| {
            TournamentFighter::ALL.map(|fighter| {
                let slug = fighter.name().to_ascii_lowercase();
                std::fs::read_to_string(format!("../../art/{directory}/{slug}.json"))
                    .expect("fighter art manifest")
            })
        };
        let recipes = read("recipes");
        let reviews = read("review");
        let pack = ArtPack::open(Path::new(ART_PACK)).expect("local art pack opens");
        let art = load_art_from(
            &assets,
            &ArtSources {
                recipes: &recipes.each_ref().map(String::as_str),
                reviews: Some(&reviews.each_ref().map(String::as_str)),
                pack: Ok(&pack),
            },
        )
        .expect("approved art loads");
        let data = Arc::new(DataBank::build(&assets.maincpu, &art.catalogues));
        (assets.with_sprite_bank(&art.bank), data)
    }))
}

fn local(
    assets: &Assets,
    bank: &Arc<DataBank>,
    fighter: TournamentFighter,
    mut config: MatchConfig,
) -> Fight {
    config.roster.mode = PlayMode::Versus;
    config.roster.p1 = fighter;
    config.roster.p2 = TournamentFighter::Ken;
    let mut fight = Fight::start_banked(
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
        Some(Arc::clone(bank)),
    )
    .expect("native local fight");
    for _ in 0..1_500 {
        step(&mut fight, assets, pad(0, None), pad(0, None));
        if flow_position(fight.machine()) == FIGHT_LOOP {
            return fight;
        }
    }
    panic!("the native fight did not reach its first round");
}

fn step(fight: &mut Fight, assets: &Assets, p1: PlayerInputs, p2: PlayerInputs) {
    fight.step(assets, p1, p2, false);
}

fn stand_close(fight: &mut Fight, assets: &Assets) {
    for _ in 0..100 {
        let telemetry = fight.machine().diagnostic_telemetry();
        if telemetry.p1.center_x.abs_diff(telemetry.p2.center_x) <= 41 {
            for _ in 0..8 {
                step(fight, assets, pad(0, None), pad(0, None));
            }
            return;
        }
        step(fight, assets, pad(RIGHT, None), pad(LEFT, None));
    }
    panic!("the fighters did not enter close range");
}

#[test]
fn all_twelve_fighters_block_high_and_low_and_tech_on_the_native_engine() {
    let Some((assets, bank)) = assets_and_bank() else {
        return;
    };
    for fighter in TournamentFighter::ALL {
        let mut fight = local(assets, bank, fighter, MatchConfig::definitive());
        assert_eq!(fight.player_fighter(0), fighter);
        stand_close(&mut fight, assets);
        let close = fight.save_netplay_snapshot(0);
        let life = fight.fight_read().fighters[0].health;

        for (direction, buttons, guard) in
            [(0, &[HP, MP, MK][..], LEFT), (DOWN, &[LK][..], LEFT | DOWN)]
        {
            fight.restore_netplay_snapshot(&close);
            let mut blocked = false;
            for _ in 0..12 {
                for &button in buttons {
                    let mut attempt = fight.clone();
                    step(
                        &mut attempt,
                        assets,
                        pad(guard, None),
                        pad(direction, Some(button)),
                    );
                    for _ in 0..26 {
                        step(&mut attempt, assets, pad(guard, None), pad(0, None));
                        let defender = attempt.fight_read().fighters[0];
                        assert_eq!(
                            defender.health, life,
                            "{fighter:?} lost life holding {guard}"
                        );
                        blocked |= defender.blockstun;
                        if blocked {
                            break;
                        }
                    }
                    if blocked {
                        break;
                    }
                }
                if blocked {
                    break;
                }
                step(&mut fight, assets, pad(0, None), pad(LEFT, None));
            }
            assert!(blocked, "{fighter:?} never blocked while holding {guard}");
        }

        fight.restore_netplay_snapshot(&close);
        for _ in 0..17 {
            let mut attempt = fight.clone();
            step(&mut attempt, assets, pad(0, None), pad(LEFT, Some(HP)));
            if attempt.fight_read().fighters[1].move_id == THROW_MOVE {
                fight = attempt;
                break;
            }
            step(&mut fight, assets, pad(0, None), pad(LEFT, None));
        }
        assert_eq!(
            fight.fight_read().fighters[1].move_id,
            THROW_MOVE,
            "{fighter:?}"
        );
        assert_eq!(
            fight.fight_read().fighters[0].move_id,
            THROWN_MOVE,
            "{fighter:?}"
        );
        let mut tech = pad(0, Some(LP));
        tech.buttons[LK] = true;
        step(&mut fight, assets, tech, pad(0, None));
        assert_eq!(
            fight.fight_read().fighters.map(|side| side.move_id),
            [1, 1],
            "{fighter:?}"
        );
        assert_eq!(fight.fight_read().fighters[0].health, life, "{fighter:?}");
    }
}

#[test]
fn corner_throw_setting_changes_spacing_after_get_up() {
    let Some((assets, bank)) = assets_and_bank() else {
        return;
    };
    let spacing = |allow_loops| {
        let mut config = MatchConfig::definitive();
        config.sf1plus.allow_corner_throw_loops = allow_loops;
        let mut fight = local(assets, bank, TournamentFighter::Retsu, config);
        // Retsu's forward throw is a front slam, so Ken stays at the right wall.
        for frame in 0..720 {
            let t = fight.machine().diagnostic_telemetry();
            if t.p2.center_x >= 0x184 && t.p1.center_x.abs_diff(t.p2.center_x) <= 39 {
                break;
            }
            let together = t.p1.center_x.abs_diff(t.p2.center_x) <= u32::from(RANGE);
            let moving = frame % 4 != 3;
            step(
                &mut fight,
                assets,
                pad(if moving { RIGHT } else { 0 }, None),
                pad(if moving && together { RIGHT } else { 0 }, None),
            );
        }
        stand_close(&mut fight, assets);
        let before = fight.machine().diagnostic_telemetry();
        assert!(
            before.p2.center_x >= 0x17a,
            "Ken reached the corner: {}",
            before.p2.center_x
        );
        step(&mut fight, assets, pad(RIGHT, Some(HP)), pad(0, None));
        assert_eq!(fight.fight_read().fighters[1].move_id, THROWN_MOVE);
        for _ in 0..120 {
            step(&mut fight, assets, pad(RIGHT, None), pad(0, None));
            if fight.machine().toggle_state().throw.wakeup_invuln[1] > 0 {
                let telemetry = fight.machine().diagnostic_telemetry();
                return (telemetry.p1.center_x, telemetry.p2.center_x);
            }
        }
        panic!("Ken did not get up from the throw");
    };
    let allowed = spacing(true);
    let prevented = spacing(false);
    assert!(
        allowed.0.abs_diff(allowed.1) <= u32::from(RANGE),
        "allowing loops leaves Retsu within throw range: {allowed:?}"
    );
    assert!(
        prevented.0.abs_diff(prevented.1) > u32::from(RANGE),
        "the default opens space at the wall: {prevented:?}"
    );
}

#[test]
fn arcade_rules_for_classic_damage_and_throw_tech_survive_settings_reload() {
    let config = MatchConfig::from_toml(&MatchConfig::definitive().to_toml()).unwrap();
    assert!(!config.sf1plus.classic_1987_special_damage);
    assert!(config.sf1plus.disable_freeze_input_lockout);
    assert!(!config.sf1plus.untechable_sf2_throws);
    assert!(!config.sf1plus.allow_corner_throw_loops);
}
