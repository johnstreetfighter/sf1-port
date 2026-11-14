//! Source-revision Ryu pacing witnesses for spec #138.

#[path = "support/alpha.rs"]
mod support;

use sf_engine::era::{loadouts, GaugeMode, Loadout, Revision};
use sf_engine::fighter::P1_STRUCT;
use sf_engine::front::fight::Fight;
use sf_engine::overlay::FighterTelemetry;
use sf_video::callouts::Side;
use sf_video::romset::Assets;
use support::{local_fight, local_select_with_p1, pad, DOWN, LEFT, LP, RIGHT, UP};

const FAR_STANDING_LP: u8 = 0x0d;
const HADOKEN: u8 = 0x1a;
const WALK_FORWARD: u8 = 2;
const WALK_BACK: u8 = 3;
const ORIGINAL_WALK: [i32; 3] = [38, 38, 40];

fn world_x(fight: &Fight, fighter: FighterTelemetry) -> i32 {
    let machine = fight.machine();
    let work = machine.work_ram();
    let camera = u16::from_be_bytes([work[0x400c], work[0x400d]]);
    let pose_offset = machine.state.objectram[usize::from(fighter.entity_offset) + 0x26] as i8;
    i32::from(camera) + fighter.center_x - i32::from(pose_offset)
}

fn back_up(fight: &mut Fight, assets: &Assets) {
    for _ in 0..42 {
        fight.step(assets, pad(LEFT, None), pad(RIGHT, None), false);
    }
    for _ in 0..18 {
        fight.step(assets, pad(0, None), pad(0, None), false);
    }
}

fn walk_distances(fight: &mut Fight, assets: &Assets, check_release: bool) -> [i32; 3] {
    let positions: Vec<_> = (0..=28)
        .map(|_| {
            fight.step(assets, pad(RIGHT, None), pad(0, None), false);
            let read = fight.machine().diagnostic_telemetry();
            assert!(!read.p1.facing_left);
            assert!(read.p1.pushbox.right < read.p2.pushbox.left);
            world_x(fight, read.p1)
        })
        .collect();
    if check_release {
        for _ in 0..4 {
            fight.step(assets, pad(0, None), pad(0, None), false);
            let ryu = fight.machine().diagnostic_telemetry().p1;
            assert_eq!(world_x(fight, ryu), positions[28], "walk stops on release");
        }
    }
    [4, 8, 12].map(|start| positions[start + 16] - positions[start])
}

fn jab(fight: &mut Fight, assets: &Assets) -> (usize, usize, usize) {
    let mut active = None;
    let mut active_ticks = 0;
    for tick in 0..24 {
        let input = if tick == 0 {
            pad(0, Some(LP))
        } else {
            pad(LEFT, None)
        };
        fight.step(assets, input, pad(0, None), false);
        let read = fight.fight_read();
        assert!(!read.connected(Side::P1), "jab must whiff");
        let ryu = read.fighters[0];
        if ryu.move_id == FAR_STANDING_LP && ryu.attack_active {
            active.get_or_insert(tick);
            active_ticks += 1;
        }
        if ryu.move_id == WALK_BACK {
            return (
                active.expect("whiffed standing LP became active"),
                active_ticks,
                tick,
            );
        }
    }
    panic!("held retreat never took over from standing LP");
}

fn fireball(fight: &mut Fight, assets: &Assets, punch: usize) -> usize {
    let mut spawned = None;
    let mut recovered = None;
    let health = fight.fight_read().fighters[1].health;
    for tick in 0..160 {
        let ryu = match tick {
            0 => pad(DOWN, None),
            1 => pad(DOWN | RIGHT, None),
            2 => pad(RIGHT, Some(punch)),
            _ => pad(LEFT, None),
        };
        let ken = pad(
            if (15..=26).contains(&tick) {
                UP | LEFT
            } else {
                0
            },
            None,
        );
        fight.step(assets, ryu, ken, false);
        let read = fight.fight_read();
        assert_eq!(read.fighters[1].health, health, "Hadoken must whiff");
        assert!(!read.connected(Side::P1));
        if read.fighters[0].move_id == HADOKEN && spawned.is_none() {
            let telemetry = fight.machine().diagnostic_telemetry();
            if telemetry
                .projectiles
                .iter()
                .any(|p| p.owner_struct == P1_STRUCT)
            {
                spawned = Some(tick);
            }
        }
        if let Some(spawn) = spawned {
            if read.fighters[0].move_id == WALK_BACK {
                recovered.get_or_insert(tick - spawn);
            }
        }
    }
    recovered.expect("held retreat took over after a live light Hadoken")
}

#[test]
fn super_turbo_ryu_matches_world_pacing_targets() {
    for (loadout, recovery) in [(Loadout::Standard, 32), (Loadout::Old, 31)] {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::St, loadout)
        else {
            return;
        };
        for opponent in [Revision::St, Revision::Sf1Plus] {
            let session = local_fight(&select, opponent, Loadout::Standard);
            let assets = session.assets();
            let mut fight = session.fight().expect("local fight").clone();
            assert_eq!(fight.fighter_style(0).unwrap().loadout, loadout);
            back_up(&mut fight, assets);

            let distances = walk_distances(&mut fight.clone(), assets, true);
            let (active, active_ticks, retreat) = jab(&mut fight.clone(), assets);
            let hadoken = fireball(&mut fight, assets, LP);
            let context = format!("{loadout:?} Ryu vs {opponent:?} Ken");
            eprintln!(
                "{context}: walk {distances:?} px/16 ticks, jab {active}/{active_ticks}/{retreat}, Hadoken {hadoken}"
            );
            assert_eq!(distances, [63; 3], "{context}: walk px/16 ticks");
            // Elapsed ticks from the first sampled LP to the live box and
            // accepted retreat, plus the number of live samples.
            assert_eq!((active, active_ticks, retreat), (2, 3, 9), "{context}: jab");
            assert_eq!(hadoken, recovery, "{context}: Hadoken recovery");
            assert!(fight.machine().unrecovered().is_empty());
        }
    }
}

#[test]
fn ryu_matches_the_other_source_revision_pacing_targets() {
    // Pixels per 16 port ticks, jab (first live, active count, first retreat),
    // and ticks from a live LP projectile to first retreat. Arcade ranges use
    // a representative observed phase. SFIV uses jab proximity to scale walk.
    // SFV and SF6 use standing pushbox widths.
    let targets = [
        (Revision::Sf1Plus, ORIGINAL_WALK, (3, 2, 7), 57),
        (Revision::Ww, [48; 3], (3, 4, 12), 40),
        (Revision::Ce, [48; 3], (3, 4, 12), 40),
        (Revision::Hf, [69; 3], (2, 3, 8), 29),
        (Revision::Super, [48; 3], (3, 4, 12), 40),
        (Revision::Sfz1, [60; 3], (2, 3, 10), 34),
        (Revision::Sfz2, [63; 3], (2, 3, 9), 31),
        (Revision::Sfz2A, [63; 3], (2, 3, 9), 32),
        (Revision::Sfz3, [60; 3], (2, 3, 11), 37),
        (Revision::Sf3Ng, [52; 3], (3, 3, 11), 43),
        (Revision::Sf3TwoI, [52; 3], (4, 2, 11), 43),
        (Revision::Sf3ThreeS, [52; 3], (4, 3, 11), 37),
        (Revision::Sf4, [38; 3], (3, 3, 12), 33),
        (Revision::Ssf4, [38; 3], (3, 3, 12), 33),
        (Revision::Ae2012, [38; 3], (3, 3, 12), 33),
        (Revision::Usf4, [38; 3], (3, 3, 12), 33),
        (Revision::Sf5, [63; 3], (2, 2, 11), 33),
        (Revision::Sf5Ae, [63; 3], (2, 2, 11), 34),
        (Revision::Sf5Ce, [63; 3], (2, 2, 11), 32),
        (Revision::Sf6, [45; 3], (3, 3, 13), 32),
    ];
    for (revision, walk_target, jab_target, hadoken_target) in targets {
        for loadout in loadouts(revision) {
            let Some(select) = local_select_with_p1(GaugeMode::EraDefault, revision, loadout)
            else {
                return;
            };
            let session = local_fight(&select, revision, loadout);
            let assets = session.assets();
            let mut fight = session.fight().expect("local fight").clone();
            let style = fight.fighter_style(0).expect("Ryu's selected style");
            assert_eq!((style.revision, style.loadout), (revision, loadout));
            let context = format!("{revision:?} {loadout:?} Ryu vs Ken");
            back_up(&mut fight, assets);

            let walk_target = match (revision, loadout) {
                (Revision::Sfz3, Loadout::XIsm) => [61; 3],
                (Revision::Sfz3, Loadout::VIsm) => [62; 3],
                _ => walk_target,
            };
            let walks = walk_distances(&mut fight.clone(), assets, revision != Revision::Sf1Plus);
            assert_eq!(walks, walk_target, "{context}: walk");
            assert_eq!(
                jab(&mut fight.clone(), assets),
                jab_target,
                "{context}: jab"
            );
            assert_eq!(
                fireball(&mut fight, assets, LP),
                hadoken_target,
                "{context}: Hadoken"
            );
            assert!(fight.machine().unrecovered().is_empty(), "{context}");
        }
    }
}

#[test]
fn medium_hadoken_keeps_its_native_recovery() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::Sfz3, Loadout::AIsm)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sfz3, Loadout::AIsm);
    let mut fight = session.fight().expect("local fight").clone();
    back_up(&mut fight, session.assets());
    assert_eq!(fireball(&mut fight, session.assets(), 1), 57);
}

#[test]
fn jab_uses_the_source_distinction_between_single_and_far_normals() {
    for (revision, loadout, single_jab) in [
        (Revision::Sfz2, Loadout::Standard, true),
        (Revision::Sfz3, Loadout::AIsm, false),
        (Revision::Sf6, Loadout::Standard, true),
    ] {
        let Some(select) = local_select_with_p1(GaugeMode::EraDefault, revision, loadout) else {
            return;
        };
        let session = local_fight(&select, revision, loadout);
        let assets = session.assets();
        let mut fight = session.fight().expect("local fight").clone();
        let gap = |fight: &Fight| {
            let read = fight.machine().diagnostic_telemetry();
            world_x(fight, read.p1).abs_diff(world_x(fight, read.p2))
        };
        // Read the selected move stream on the LP sample, before contact can
        // advance or freeze it. The separate whiff tests time the far stream.
        let jab_stream = |fight: &mut Fight| {
            fight.step(assets, pad(0, Some(LP)), pad(0, None), false);
            let ryu = fight.machine().diagnostic_telemetry().p1;
            assert_eq!(ryu.move_id, FAR_STANDING_LP);
            let pointer = usize::from(ryu.entity_offset) + 0x2c;
            let objects = &fight.machine().state.objectram;
            u32::from_be_bytes(objects[pointer..pointer + 4].try_into().unwrap())
        };
        let mut far = fight.clone();
        back_up(&mut far, assets);
        assert!(gap(&far) > 64, "{revision:?}: far range");
        let far_stream = jab_stream(&mut far);
        assert!(sf_engine::data_bank::in_bank(far_stream));
        for _ in 0..60 {
            if gap(&fight) <= 60 {
                break;
            }
            fight.step(assets, pad(RIGHT, None), pad(0, None), false);
        }
        fight.step(assets, pad(0, None), pad(0, None), false);
        assert!(gap(&fight) <= 64, "{revision:?}: close range");
        assert_eq!(
            jab_stream(&mut fight) == far_stream,
            single_jab,
            "{revision:?}"
        );
    }
}

#[test]
fn ryu_pacing_replays_from_mid_move_snapshots_in_a_mixed_era_fight() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::Sfz3, Loadout::VIsm)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut fight = session.fight().expect("local fight").clone();
    back_up(&mut fight, assets);

    // Snapshot during the fractional walk cadence, the live jab, and the
    // projectile recovery. Each replay must restore the complete native state.
    for tick in 0..100 {
        let ryu = match tick {
            0..=21 => pad(RIGHT, None),
            22 => pad(0, Some(LP)),
            40 => pad(DOWN, None),
            41 => pad(DOWN | RIGHT, None),
            42 => pad(RIGHT, Some(LP)),
            _ => pad(0, None),
        };
        let ken = pad(
            if (54..=65).contains(&tick) {
                UP | LEFT
            } else {
                0
            },
            None,
        );
        let move_at_snapshot = match tick {
            7 => Some(WALK_FORWARD),
            25 => Some(FAR_STANDING_LP),
            59 => Some(HADOKEN),
            _ => None,
        };
        let before = move_at_snapshot.map(|move_id| {
            assert_eq!(fight.fight_read().fighters[0].move_id, move_id);
            fight.save_netplay_snapshot(tick)
        });
        fight.step(assets, ryu, ken, false);
        if let Some(before) = before {
            let after = fight.save_netplay_snapshot(tick + 1);
            let checksum = fight.state_checksum();
            fight.restore_netplay_snapshot(&before);
            fight.step(assets, ryu, ken, false);
            assert_eq!(fight.state_checksum(), checksum, "replay at tick {tick}");
            assert!(
                fight.save_netplay_snapshot(tick + 1) == after,
                "replay at tick {tick}"
            );
        }
    }
}
