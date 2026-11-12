//! Native Fight checks for World Tour combat modifiers.
use std::path::Path;
use std::sync::Arc;

use sf_engine::config::{MatchConfig, WorldTourBonuses, WorldTourRules};
use sf_engine::data_bank::DataBank;
use sf_engine::era::{loadouts, Revision, StylePick};
use sf_engine::front::cabinet::{flow_position, Players};
use sf_engine::front::fight::{Fight, FightDips, FightKind, FightSetup};
use sf_engine::front::PlayerInputs;
use sf_engine::roster::TournamentFighter;
use sf_engine::world_tour::native::{MissionKind, Perk, RuleModifier};
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::romset::{load_assets, Assets};

fn ready_fight(rules: WorldTourRules) -> (Assets, Fight) {
    ready_fight_in_style(rules, Revision::Sf1Plus)
}

fn ready_fight_in_style(rules: WorldTourRules, revision: Revision) -> (Assets, Fight) {
    ready_fight_against(rules, revision, TournamentFighter::Ken)
}

fn ready_fight_against(
    rules: WorldTourRules,
    revision: Revision,
    opponent: TournamentFighter,
) -> (Assets, Fight) {
    let assets = load_assets(
        "../../tools/reference/local/roms/sf",
        "../../tools/reference/rom-manifest.json",
    )
    .expect("World Tour integration tests require the verified local ROM set");
    let catalogues = if opponent == TournamentFighter::Geki {
        let pack = ArtPack::open(Path::new("../../assets/sf1-port-art")).expect("bundled art pack");
        let recipes = [
            include_str!("../../../art/recipes/ryu.json"),
            include_str!("../../../art/recipes/geki.json"),
        ];
        let reviews = [
            include_str!("../../../art/review/ryu.json"),
            include_str!("../../../art/review/geki.json"),
        ];
        load_art_from(
            &assets,
            &ArtSources {
                recipes: &recipes,
                reviews: Some(&reviews),
                pack: Ok(&pack),
            },
        )
        .expect("Geki's art loads")
        .catalogues
    } else {
        Vec::new()
    };
    let data = Arc::new(DataBank::build(&assets.maincpu, &catalogues));
    let mut config = MatchConfig::definitive();
    config.world_tour = Some(rules);
    config.roster.p2 = opponent;
    let setup = FightSetup {
        kind: FightKind::WorldTour,
        players: Players::Two,
        config,
        dsw1: FightDips::DSW1,
        dsw2: FightDips::default().dsw2(),
    };
    let mut fight = Fight::start_banked(&assets.maincpu, setup, Some(0), None, Some(data))
        .expect("World Tour starts the native cabinet");
    let pick = StylePick {
        family: revision.family(),
        revision,
        loadout: loadouts(revision)[0],
        ..StylePick::default()
    };
    let mut machine = fight.machine().clone();
    machine.set_era_styles([pick; 2]);
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine = machine.save_snapshot();
    fight.restore_netplay_snapshot(&snapshot);
    for _ in 0..1500 {
        if flow_position(fight.machine()) == 0x3eafa && !fight.driving() {
            return (assets, fight);
        }
        step(&assets, &mut fight);
    }
    panic!("World Tour never entered the native fight loop");
}

fn step(assets: &Assets, fight: &mut Fight) {
    let frame = fight.step(
        assets,
        PlayerInputs::default(),
        PlayerInputs::default(),
        false,
    );
    assert!(frame.event.is_none(), "{:?}", frame.event);
}

fn native_punch(assets: &Assets, fight: &mut Fight, side: usize) {
    set_punch(fight, side, true);
    step(assets, fight);
}

fn set_punch(fight: &mut Fight, side: usize, active: bool) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    let objects = &mut snapshot.machine.state.objectram;
    let attacker = [0x400, 0xa00][side];
    // A native light punch on its active frame with a box that reaches the
    // opponent at the opening position.
    objects[attacker + 9] = 0xc2;
    objects[attacker + 0xa] = 0x0e;
    objects[attacker + 0xf] = if active { 0x80 } else { 0 };
    objects[attacker + 0x22..attacker + 0x26].copy_from_slice(&[0, 0xff, 0, 0xff]);
    objects[attacker + 0x28..attacker + 0x2c].copy_from_slice(&0x45af0u32.to_be_bytes());
    objects[attacker + 0x33] = 2;
    objects[[0xa00, 0x400][side] + 0xf] &= !0x08;
    fight.restore_netplay_snapshot(&snapshot);
}

fn punch_damage(bonuses: Option<WorldTourBonuses>, side: usize) -> u16 {
    let (assets, mut fight) = ready_fight(WorldTourRules {
        mission: MissionKind::RivalBout,
        bonuses,
        starting_hp: None,
    });
    let before = fight.machine().era_fighters()[1 - side].sub_hp;
    native_punch(&assets, &mut fight, side);
    before - fight.machine().era_fighters()[1 - side].sub_hp
}

#[test]
fn native_punches_apply_attack_and_defense_only_to_the_world_tour_player() {
    let bonuses = Some(WorldTourBonuses {
        attack: 5,
        defense: 5,
        speed: 0,
        perk: None,
    });
    let outgoing = punch_damage(None, 0);
    let incoming = punch_damage(None, 1);
    assert!(outgoing > 0 && incoming > 0);
    assert_eq!(punch_damage(bonuses, 0), outgoing + outgoing / 5);
    assert_eq!(punch_damage(bonuses, 1), incoming - incoming / 5);
}

fn bonuses(perk: Perk) -> Option<WorldTourBonuses> {
    Some(WorldTourBonuses {
        attack: 0,
        defense: 0,
        speed: 0,
        perk: Some(perk),
    })
}

fn trial(rule: RuleModifier) -> WorldTourRules {
    WorldTourRules {
        mission: MissionKind::RuleModifierTrial(rule),
        bonuses: None,
        starting_hp: None,
    }
}

#[test]
fn combo_only_openers_stun_and_award_meter_without_draining_health() {
    let (assets, mut fight) = ready_fight_in_style(trial(RuleModifier::ComboOnly), Revision::St);
    let before = fight.machine().era_fighters()[1].sub_hp;
    let meter = fight.machine().era_fighters()[0].super_meter;
    native_punch(&assets, &mut fight, 0);
    assert_eq!(fight.machine().era_fighters()[1].sub_hp, before);
    assert!(fight.machine().toggle_state().combo.hitstun[1] > 0);
    assert!(fight.machine().era_fighters()[0].super_meter > meter);
    native_punch(&assets, &mut fight, 0);
    assert!(fight.machine().era_fighters()[1].sub_hp < before);
}

fn dirs(indices: &[usize]) -> PlayerInputs {
    let mut pad = PlayerInputs::default();
    for &direction in indices {
        pad.directions[direction] = true;
    }
    pad
}

#[test]
fn no_blocking_stops_both_ground_guards_without_disabling_the_opponent_guard() {
    let rules = trial(RuleModifier::NoBlocking);
    let (assets, fight) = ready_fight(rules);
    let (_, control) = ready_fight(WorldTourRules {
        mission: MissionKind::RivalBout,
        ..rules
    });
    for (base, defender) in [(fight.clone(), 0), (control, 0), (fight, 1)] {
        for low in [false, true] {
            let mut attempt = base.clone();
            set_punch(&mut attempt, 1 - defender, false);
            let mut pad = dirs(&[1 - defender]);
            pad.directions[2] = low;
            let mut inputs = [PlayerInputs::default(); 2];
            inputs[defender] = pad;
            attempt.step(&assets, inputs[0], inputs[1], false);
            let reads = attempt.machine().diagnostic_telemetry();
            let mv = [reads.p1.move_id, reads.p2.move_id][defender];
            let allowed =
                base.setup().config.world_tour.unwrap().mission != rules.mission || defender == 1;
            assert_eq!(
                mv == if low { 8 } else { 4 },
                allowed,
                "defender {defender}, low {low}, move {mv:02x}"
            );
        }
    }
}

#[test]
fn world_tour_speed_increases_native_walk_distance() {
    fn distance(speed: u8) -> i32 {
        let (assets, mut fight) = ready_fight(WorldTourRules {
            mission: MissionKind::RivalBout,
            bonuses: Some(WorldTourBonuses {
                attack: 0,
                defense: 0,
                speed,
                perk: None,
            }),
            starting_hp: None,
        });
        let gap = |fight: &Fight| {
            let read = fight.machine().diagnostic_telemetry();
            read.p2.center_x - read.p1.center_x
        };
        let initial_gap = gap(&fight);
        for _ in 0..24 {
            fight.step(&assets, dirs(&[0]), PlayerInputs::default(), false);
        }
        initial_gap - gap(&fight)
    }
    let normal = distance(0);
    let boosted = distance(5);
    assert!(normal > 0);
    assert!(boosted >= normal + normal / 8, "{normal} -> {boosted}");
    assert!(boosted <= normal + normal / 5, "{normal} -> {boosted}");
}

#[test]
fn meter_surge_starts_p1_with_a_stock_and_accumulates_quarter_point_gains() {
    // SFZ3 A-ISM has three 48-point stocks.
    let rules = WorldTourRules {
        mission: MissionKind::RivalBout,
        bonuses: bonuses(Perk::MeterSurge),
        starting_hp: None,
    };
    let (assets, mut surge) = ready_fight_in_style(rules, Revision::Sfz3);
    let (_, mut base) = ready_fight_in_style(
        WorldTourRules {
            bonuses: None,
            ..rules
        },
        Revision::Sfz3,
    );
    assert_eq!(surge.machine().era_fighters()[0].super_meter, 48);
    assert_eq!(surge.machine().era_fighters()[1].super_meter, 0);
    native_punch(&assets, &mut surge, 0);
    native_punch(&assets, &mut base, 0);
    let unboosted = base.machine().era_fighters()[0].super_meter;
    assert!(unboosted > 0);
    assert_eq!(
        surge.machine().era_fighters()[0].super_meter,
        48 + unboosted + unboosted / 4
    );
}

#[test]
fn survival_health_is_carried_into_native_ram_and_vital_spirit_recovers_it() {
    let rules = WorldTourRules {
        mission: MissionKind::SurvivalGauntlet {
            recovery_percent: 20,
        },
        bonuses: bonuses(Perk::VitalSpirit),
        starting_hp: Some(440),
    };
    let (assets, mut fight) = ready_fight(rules);
    assert_eq!(fight.machine().era_fighters()[0].sub_hp, 440);
    let player = (sf_engine::fighter::P1_STRUCT - 0xff8000) as usize;
    let native_hp = &fight.machine().work_ram()[player + 6..player + 8];
    assert_eq!(u16::from(native_hp[0]) + 4 * u16::from(native_hp[1]), 22);
    let bar = &fight.machine().state.videoram[0x2bc..0x2d4];
    let cells: Vec<_> = bar
        .chunks_exact(2)
        .map(|b| u16::from_be_bytes([b[0], b[1]]))
        .collect();
    assert_eq!(
        cells,
        [
            0x1120, 0x1120, 0x1120, 0x1120, 0x1120, 0x1122, 0x1124, 0x1124, 0x1124, 0x1124, 0x1124,
            0x1124
        ]
    );
    for _ in 0..30 {
        step(&assets, &mut fight);
        if fight.machine().era_fighters()[0].sub_hp != 440 {
            break;
        }
    }
    assert_eq!(fight.machine().era_fighters()[0].sub_hp, 442);
    for _ in 0..29 {
        step(&assets, &mut fight);
    }
    assert_eq!(fight.machine().era_fighters()[0].sub_hp, 442);
    step(&assets, &mut fight);
    assert_eq!(fight.machine().era_fighters()[0].sub_hp, 444);
}

fn shot_pad(frame: usize, forward: usize) -> PlayerInputs {
    let mut pad = match frame {
        0..=2 => dirs(&[2]),
        3..=5 => dirs(&[2, forward]),
        6..=8 => dirs(&[forward]),
        _ => PlayerInputs::default(),
    };
    pad.buttons[0] = (6..=8).contains(&frame);
    pad
}

#[test]
fn pure_footsies_replaces_both_projectile_commands_with_normals() {
    let rules = trial(RuleModifier::PureFootsies);
    let (assets, pure) = ready_fight(rules);
    let (_, control) = ready_fight(WorldTourRules {
        mission: MissionKind::RivalBout,
        ..rules
    });
    for (base, allow_special) in [(pure, false), (control, true)] {
        for side in 0..2 {
            let mut fight = base.clone();
            let mut seen_moves = Vec::new();
            for frame in 0..35 {
                let mut inputs = [PlayerInputs::default(); 2];
                inputs[side] = shot_pad(frame, side);
                fight.step(&assets, inputs[0], inputs[1], false);
                let reads = fight.machine().diagnostic_telemetry();
                seen_moves.push([reads.p1.move_id, reads.p2.move_id][side]);
            }
            assert_eq!(seen_moves.contains(&0x1a), allow_special, "{seen_moves:?}");
            if !allow_special {
                assert!(
                    seen_moves.iter().any(|mv| (0x0d..=0x18).contains(mv)),
                    "{seen_moves:?}"
                );
            }
        }
    }
}

#[test]
fn guard_master_eliminates_native_projectile_chip() {
    let rules = WorldTourRules {
        mission: MissionKind::RivalBout,
        bonuses: bonuses(Perk::GuardMaster),
        starting_hp: None,
    };
    for (bonuses, expected) in [(None, 20), (rules.bonuses, 0)] {
        let (assets, mut fight) = ready_fight(WorldTourRules { bonuses, ..rules });
        let before = fight.machine().era_fighters()[0].sub_hp;
        let mut guarded = false;
        for frame in 0..100 {
            fight.step(&assets, dirs(&[1]), shot_pad(frame, 1), false);
            guarded |= fight.machine().diagnostic_telemetry().p1.move_id == 4;
        }
        assert!(guarded);
        assert_eq!(before - fight.machine().era_fighters()[0].sub_hp, expected);
    }
}

#[test]
fn air_guard_allows_sf1_plus_to_guard_in_the_air_but_no_blocking_overrides_it() {
    let rules = WorldTourRules {
        mission: MissionKind::RivalBout,
        bonuses: bonuses(Perk::AirGuard),
        starting_hp: None,
    };
    for (mission, bonuses, expected) in [
        (MissionKind::RivalBout, None, 180),
        (MissionKind::RivalBout, rules.bonuses, 20),
        (
            MissionKind::RuleModifierTrial(RuleModifier::NoBlocking),
            rules.bonuses,
            180,
        ),
    ] {
        let (assets, mut fight) = ready_fight(WorldTourRules {
            mission,
            bonuses,
            ..rules
        });
        let before = fight.machine().era_fighters()[0].sub_hp;
        for frame in 0..100 {
            let p1 = match frame {
                0..=19 => PlayerInputs::default(),
                20 => dirs(&[3]),
                _ => dirs(&[1]),
            };
            fight.step(&assets, p1, shot_pad(frame, 1), false);
        }
        assert_eq!(
            before - fight.machine().era_fighters()[0].sub_hp,
            expected,
            "{mission:?} {bonuses:?}"
        );
    }
}

#[test]
fn ism_plus_charges_three_quarters_of_the_native_drive_impact_cost() {
    let rules = WorldTourRules {
        mission: MissionKind::RivalBout,
        bonuses: bonuses(Perk::IsmPlus),
        starting_hp: None,
    };
    for (bonuses, expected) in [(None, 48), (rules.bonuses, 36)] {
        let (assets, mut fight) =
            ready_fight_in_style(WorldTourRules { bonuses, ..rules }, Revision::Sf6);
        let before = fight.machine().era_fighters()[0]
            .sf6_drive_points()
            .unwrap();
        let mut impact = PlayerInputs::default();
        impact.buttons[2] = true;
        impact.buttons[5] = true;
        fight.step(&assets, impact, PlayerInputs::default(), false);
        let after = fight.machine().era_fighters()[0]
            .sf6_drive_points()
            .unwrap();
        assert_eq!(before - after, expected);
    }
}

#[test]
fn projectile_defense_halves_gekis_native_recovery_after_the_shuriken_leaves_his_hand() {
    let rules = trial(RuleModifier::ProjectileDefense);
    let recovery = |mission| {
        let (assets, mut fight) = ready_fight_against(
            WorldTourRules { mission, ..rules },
            Revision::Sf1Plus,
            TournamentFighter::Geki,
        );
        let mut launch = None;
        for frame in 0..90 {
            fight.step(&assets, PlayerInputs::default(), shot_pad(frame, 1), false);
            let objects = &fight.machine().state.objectram;
            let shot_live = objects[0x100..0x400]
                .chunks_exact(0x40)
                .any(|slot| slot[8] != 0);
            if shot_live && launch.is_none() {
                launch = Some(frame);
            }
            if let Some(start) = launch {
                let entity = if objects[0x400 + 0x38..0x400 + 0x3c]
                    == sf_engine::fighter::P2_STRUCT.to_be_bytes()
                {
                    0x400
                } else {
                    0xa00
                };
                if objects[entity + 9] & 0x80 == 0 {
                    return (start, frame - start);
                }
            }
        }
        panic!("Geki's shuriken never recovered");
    };
    let (launch, regular) = recovery(MissionKind::RivalBout);
    let (trial_launch, shortened) = recovery(rules.mission);
    assert_eq!(launch, trial_launch, "startup stays on the native timing");
    assert!(regular > 10);
    assert_eq!(shortened, regular.div_ceil(2));
}

#[test]
fn attack_boost_applies_to_the_authored_damage_of_sf5_ryus_critical_art() {
    let rules = WorldTourRules {
        mission: MissionKind::RivalBout,
        bonuses: None,
        starting_hp: None,
    };
    for (attack, per_hit) in [(0, 72), (5, 86)] {
        let (assets, mut fight) = ready_fight_in_style(
            WorldTourRules {
                bonuses: Some(WorldTourBonuses {
                    attack,
                    defense: 0,
                    speed: 0,
                    perk: None,
                }),
                ..rules
            },
            Revision::Sf5Ce,
        );
        let before = fight.machine().era_fighters()[1].sub_hp;
        let mut snapshot = fight.save_netplay_snapshot(0);
        snapshot.machine.toggle_state.era.fighters[0].super_meter = 144;
        fight.restore_netplay_snapshot(&snapshot);
        let mut button = dirs(&[0]);
        button.buttons[2] = true;
        for p1 in [
            dirs(&[2]),
            dirs(&[0, 2]),
            dirs(&[0]),
            dirs(&[2]),
            dirs(&[0, 2]),
            button,
        ] {
            fight.step(&assets, p1, PlayerInputs::default(), false);
        }
        assert_eq!(fight.machine().diagnostic_telemetry().p1.move_id, 0x29);
        for _ in 0..100 {
            step(&assets, &mut fight);
        }
        let damage = before - fight.machine().era_fighters()[1].sub_hp;
        assert!(
            damage >= per_hit && damage % per_hit == 0,
            "attack level {attack} produced {damage} damage"
        );
    }
}
