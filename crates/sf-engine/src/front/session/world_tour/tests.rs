use super::*;
use crate::era::Loadout;
use crate::fighter::P1_STRUCT;
use crate::front::cabinet::flow_position;
use crate::front::{FrameInputs, MenuItem};
use crate::roster::TournamentFighter;
use crate::tasks::TIME_DIGITS;
use crate::world_tour::native::{node, NODES};
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::background_bank::BackgroundBank;
use sf_video::romset::load_assets;
use std::path::Path;
use std::sync::OnceLock;

fn session() -> GameSession {
    static SESSION: OnceLock<GameSession> = OnceLock::new();
    SESSION
        .get_or_init(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let assets = load_assets(
                root.join("tools/reference/local/roms/sf"),
                root.join("tools/reference/rom-manifest.json"),
            )
            .expect("native World Tour requires the verified local ROM set");
            let pack = ArtPack::open(&root.join("assets/sf1-port-art")).unwrap();
            let recipes = TournamentFighter::ALL
                .into_iter()
                .filter(|fighter| *fighter != TournamentFighter::Ken)
                .map(|fighter| {
                    std::fs::read_to_string(
                        root.join("art/recipes")
                            .join(format!("{}.json", fighter.name().to_ascii_lowercase())),
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>();
            let sources = ArtSources {
                recipes: &recipes.iter().map(String::as_str).collect::<Vec<_>>(),
                reviews: None,
                pack: Ok(&pack),
            };
            let art = load_art_from(&assets, &sources).unwrap();
            let mut backgrounds = BackgroundBank::new();
            backgrounds.add_rom_stages(&assets).unwrap();
            GameSession::new(assets, MatchConfig::definitive())
                .with_sprite_bank(&art.bank)
                .with_background_bank(backgrounds)
                .with_player_tables(&art.catalogues)
        })
        .clone()
}

fn tap(session: &mut GameSession, change: impl FnOnce(&mut FrameInputs)) -> &'static str {
    let mut input = FrameInputs::default();
    change(&mut input);
    let screen = session.step(input).screen;
    session.step(FrameInputs::default());
    screen
}

fn open_map(session: &mut GameSession) {
    for _ in 0..200 {
        session.step(FrameInputs::default());
    }
    assert_eq!(tap(session, |i| i.menu.start = true), "main menu");
    assert_eq!(tap(session, |i| i.menu.confirm = true), "single player");
    tap(session, |i| i.p1.directions[2] = true);
    tap(session, |i| i.p1.directions[2] = true);
    assert_eq!(session.menu_item(), Some(MenuItem::WorldTour));
    assert_eq!(tap(session, |i| i.menu.confirm = true), "world tour");
}

fn start(session: &mut GameSession, id: &str) {
    if session.world_tour_profile.current_node().id != id {
        assert!(session.world_tour_profile.select_node(id));
    }
    tap(session, |i| i.menu.confirm = true);
    assert!(
        session.fight().is_some(),
        "{}",
        session.notice().unwrap_or(id)
    );
}

fn until(session: &mut GameSession, ready: impl Fn(&GameSession) -> bool) {
    for _ in 0..8_000 {
        if ready(session) {
            return;
        }
        assert!(session
            .take_log()
            .iter()
            .all(|line| !line.contains("untranslated")));
        session.step(FrameInputs::default());
    }
    panic!(
        "World Tour stuck at {} {:?}",
        session.screen(),
        session.fight_machine().map(flow_position)
    );
}

fn in_round(session: &GameSession) -> bool {
    session.fight_machine().map(flow_position) == Some(0x3eafa)
}

fn patch_fight(
    session: &mut GameSession,
    patch: impl FnOnce(&mut crate::front::fight::FightSnapshot),
) {
    let fight = &mut session.play.as_mut().unwrap().fight;
    let mut snapshot = fight.save_netplay_snapshot(0);
    patch(&mut snapshot);
    fight.restore_netplay_snapshot(&snapshot);
}

fn time_digits(session: &GameSession) -> &[u8] {
    let offset = (TIME_DIGITS - 0xff8000) as usize;
    &session.fight_machine().unwrap().work_ram()[offset..offset + 3]
}

#[test]
fn timed_missions_use_native_time_over_with_opposite_victory_conditions() {
    let mut game = session();
    open_map(&mut game);
    for id in ["jp-1-dojo", "jp-3-era", "jp-4-boss"] {
        assert!(game.world_tour_profile.complete(node(id).unwrap()));
    }
    for (id, tens, expected) in [("cn-1-wall", 4, false), ("jp-2-shuriken", 6, true)] {
        start(&mut game, id);
        until(&mut game, in_round);
        assert_eq!(time_digits(&game), &[0, tens, 0]);
        patch_fight(&mut game, |snapshot| {
            let offset = (TIME_DIGITS - 0xff8000) as usize;
            snapshot.machine.work[offset..offset + 3].fill(0);
            snapshot.machine.toggle_state.era.fighters[1].sub_hp = 500;
        });
        if id == "jp-2-shuriken" {
            until(&mut game, |game| {
                game.fight_machine().map(flow_position) == Some(0x3ed96)
            });
            patch_fight(&mut game, |snapshot| {
                // A hit during the native result hold cannot revoke survival.
                snapshot.machine.toggle_state.era.fighters[0].sub_hp = 0;
            });
        }
        until(&mut game, |game| game.fight().is_none());
        assert_eq!(game.screen(), "world tour");
        assert_eq!(
            game.world_tour_profile.is_cleared(node(id).unwrap()),
            expected
        );
    }
}

fn before_finale(game: &mut GameSession) {
    for node in NODES.iter().take(NODES.len() - 1) {
        assert!(game.world_tour_profile.complete(node), "{}", node.id);
    }
}

fn knockout_opponent(game: &mut GameSession, remaining: u16) {
    patch_fight(game, |snapshot| {
        let machine = &mut snapshot.machine;
        if snapshot.setup.config.dramatic.is_some() {
            machine.toggle_state.dramatic.health[1] = 0;
        } else {
            machine.toggle_state.era.fighters[0].sub_hp = remaining;
            machine.toggle_state.era.fighters[1].sub_hp = 0;
            machine.state.objectram[0xa08] = 8;
            machine.state.objectram[0xa09] = 0;
        }
    });
}

#[test]
fn all_eighteen_nodes_launch_the_authored_native_matchup_and_stage() {
    let mut game = session();
    open_map(&mut game);
    before_finale(&mut game);
    game.world_tour_profile.fighter = TournamentFighter::Ken;
    game.world_tour_profile.rpg = false;
    game.world_tour_profile.style = StylePick::for_revision(Revision::Sfz3, Loadout::VIsm);
    game.config.gauge_mode = GaugeMode::NoGauges;
    for node in NODES.iter() {
        start(&mut game, node.id);
        until(&mut game, in_round);
        let fight = game.fight().unwrap();
        let config = fight.setup().config;
        let stage = game
            .backgrounds
            .as_ref()
            .unwrap()
            .stage_index(&rom_stage_name(node.stage.name()));
        assert_eq!(
            config.roster.stage,
            stage.map(StagePick::Bank),
            "{}",
            node.id
        );
        assert_eq!(config.roster.p1, TournamentFighter::Ken);
        assert_eq!(config.roster.p2, node.opponents[0]);
        assert_eq!(config.world_tour.unwrap().mission, node.mission);
        assert_eq!(config.world_tour.unwrap().bonuses, None);
        if matches!(node.mission, MissionKind::EraStyleMastery { .. }) || node.is_finale() {
            assert_eq!(config.gauge_mode, GaugeMode::EraDefault);
        }
        let fighters = fight.machine().dramatic_era_fighters();
        assert_eq!(
            fighters[0].fighter,
            Some(TournamentFighter::Ken),
            "{}",
            node.id
        );
        assert_eq!(fighters[1].fighter, Some(node.opponents[0]), "{}", node.id);
        assert_eq!(
            fighters[0].style,
            node.player_style(game.world_tour_profile.style)
        );
        if let Some(ally) = node.ally_for(TournamentFighter::Ken) {
            assert_eq!(fight.setup().kind, FightKind::Dramatic { co_op: false });
            assert_eq!(config.dramatic.unwrap().ally2, ally);
            assert_eq!(fighters[2].fighter, Some(ally));
        } else {
            assert_eq!(fight.setup().kind, FightKind::WorldTour);
            assert!(config.p2_fair_cpu);
        }
        game.leave_world_tour_fight();
    }
}

#[test]
fn survival_carries_native_life_with_the_authored_recovery_and_cap() {
    let mut game = session();
    open_map(&mut game);
    for id in [
        "jp-1-dojo",
        "jp-3-era",
        "jp-4-boss",
        "cn-1-wall",
        "cn-3-era",
        "cn-4-boss",
    ] {
        assert!(game.world_tour_profile.complete(node(id).unwrap()));
    }
    start(&mut game, "en-1-punk");
    until(&mut game, in_round);
    for (remaining, next, expected) in [
        (400, TournamentFighter::Birdie, 592),
        (900, TournamentFighter::Eagle, 960),
    ] {
        knockout_opponent(&mut game, remaining);
        until(&mut game, |game| {
            in_round(game) && game.fight().unwrap().setup().config.roster.p2 == next
        });
        let machine = game.fight_machine().unwrap();
        assert_eq!(machine.era_fighters()[0].sub_hp, expected);
        let at = (P1_STRUCT - 0xff8000) as usize;
        let ram = machine.work_ram();
        assert_eq!(
            u16::from(ram[at + 6]) + 4 * u16::from(ram[at + 7]),
            expected.div_ceil(20)
        );
    }
    knockout_opponent(&mut game, 550);
    until(&mut game, |game| game.fight().is_none());
    assert_eq!(game.screen(), "world tour");
    assert!(
        game.world_tour_profile
            .is_cleared(node("en-1-punk").unwrap()),
        "the final wave recorded the clear"
    );

    for id in ["en-2-footsies", "en-4-boss"] {
        assert!(game.world_tour_profile.complete(node(id).unwrap()));
    }
    start(&mut game, "th-1-gauntlet");
    until(&mut game, in_round);
    for next in [
        TournamentFighter::Gen,
        TournamentFighter::Eagle,
        TournamentFighter::Adon,
    ] {
        knockout_opponent(&mut game, 500);
        until(&mut game, |game| {
            in_round(game) && game.fight().unwrap().setup().config.roster.p2 == next
        });
        assert_eq!(game.fight_machine().unwrap().era_fighters()[0].sub_hp, 740);
    }
    knockout_opponent(&mut game, 500);
    until(&mut game, |game| game.fight().is_none());
    assert!(game
        .world_tour_profile
        .is_cleared(node("th-1-gauntlet").unwrap()));
}

#[test]
fn thailand_clear_plays_native_message_and_staff_roll_then_returns_to_map() {
    let mut game = session();
    open_map(&mut game);
    for id in [
        "jp-1-dojo",
        "jp-3-era",
        "jp-4-boss",
        "cn-1-wall",
        "cn-3-era",
        "cn-4-boss",
        "en-1-punk",
        "en-2-footsies",
        "en-4-boss",
    ] {
        assert!(game.world_tour_profile.complete(node(id).unwrap()));
    }
    assert!(!game
        .world_tour_profile
        .is_unlocked(node("us-2-rush").unwrap()));
    game.world_tour_profile.fighter = TournamentFighter::Ken;
    start(&mut game, "th-2-era");
    until(&mut game, in_round);
    patch_fight(&mut game, |snapshot| {
        let at = (P1_STRUCT - 0xff8000) as usize;
        snapshot.machine.work[at + 8] = 1;
    });
    knockout_opponent(&mut game, 550);
    until(&mut game, |game| {
        let Some(machine) = game.fight_machine() else {
            return false;
        };
        assert!(
            machine.task_positions().iter().all(|&(slot, pc)| {
                slot != 2 || !matches!(pc, 0x401e6 | 0x401ec | 0x4029a | 0x400c2 | 0x400d4)
            }),
            "a World Tour result must not show an unrelated Arcade or versus card"
        );
        flow_position(machine) == 0x44b20
    });
    assert!(
        game.world_tour_profile
            .is_cleared(node("th-2-era").unwrap()),
        "the finale recorded the clear"
    );
    assert!(
        game.take_world_tour_changed(),
        "the host receives the clear before the credits finish"
    );
    until(&mut game, |game| {
        game.fight_machine().map(flow_position) == Some(0x44c04)
    });
    until(&mut game, |game| game.fight().is_none());
    assert_eq!(game.screen(), "world tour");
    for node in NODES.iter() {
        if game.world_tour_profile.current_node().id != node.id {
            assert!(game.world_tour_profile.select_node(node.id), "{}", node.id);
        }
    }
}

#[test]
fn rival_bout_requires_two_native_round_wins_before_awarding_xp() {
    let mut game = session();
    open_map(&mut game);
    start(&mut game, "jp-1-dojo");
    until(&mut game, in_round);
    knockout_opponent(&mut game, 300);
    game.step(FrameInputs::default());
    until(&mut game, |game| {
        in_round(game) && game.fight_machine().unwrap().work_ram()[0x400a] == 1
    });
    let dojo = node("jp-1-dojo").unwrap();
    assert!(!game.world_tour_profile.is_cleared(dojo));
    assert_eq!(game.world_tour_profile.xp(), 0);
    knockout_opponent(&mut game, 500);
    until(&mut game, |game| game.fight().is_none());
    assert!(game.world_tour_profile.is_cleared(dojo));
    assert_eq!(game.world_tour_profile.xp(), dojo.xp_reward);
    assert!(game.take_world_tour_changed());
}
