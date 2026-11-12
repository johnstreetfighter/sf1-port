//! ADR-0015 through the ordinary menu, native fight, and rollback entry points.

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::era::{Loadout, Revision};
use sf_engine::era_fight::{FULL_SUB_HP, SF2_SUPER, SUPER_FREEZE_FRAMES};
use sf_engine::fighter::{P1_STRUCT, P2_STRUCT};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::fight::Fight;
use sf_engine::front::menu::MenuItem;
use sf_engine::front::settings::Settings;
use sf_engine::front::{FrameInputs, GameSession, PlayerInputs, Transition};
use sf_engine::netplay::{encode_inputs, RollbackEngine};
use sf_engine::roster::TournamentFighter;
use sf_video::announcer::AnnouncerLine;
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const GAUGE_FULL: u8 = 48;

fn assets() -> Assets {
    load_assets(ROMS, MANIFEST).expect("local ROMs assemble")
}

fn tap(session: &mut GameSession, input: FrameInputs) -> Option<Transition> {
    let output = session.step(input);
    session.step(FrameInputs::default());
    output.transition
}

fn pad(dir: u8, punch: bool) -> PlayerInputs {
    let mut input = PlayerInputs {
        directions: [1, 2, 4, 8].map(|bit| dir & bit != 0),
        ..PlayerInputs::default()
    };
    input.buttons[0] = punch;
    input
}

fn side_tap(session: &mut GameSession, side: usize, dir: u8, punch: bool) -> Option<Transition> {
    let mut input = FrameInputs::default();
    if side == 0 {
        input.p1 = pad(dir, punch);
    } else {
        input.p2 = pad(dir, punch);
    }
    tap(session, input)
}

fn choose(session: &mut GameSession, item: MenuItem) {
    for _ in 0..8 {
        if session.menu_item() == Some(item) {
            assert_eq!(
                side_tap(session, 0, 0, true),
                Some(Transition::Choose(item))
            );
            return;
        }
        side_tap(session, 0, 4, false);
    }
    panic!("no {item:?} on {}", session.screen());
}

fn select(session: &mut GameSession, side: usize, style: Option<(Revision, Loadout)>) {
    assert_eq!(
        side_tap(session, side, 0, true),
        Some(Transition::StyleOpened)
    );
    if let Some((revision, loadout)) = style {
        side_tap(session, side, 1, false); // SF2 ST
        if revision == Revision::Sfz3 {
            assert_eq!(loadout, Loadout::AIsm);
            side_tap(session, side, 1, false); // SFZ3 A-ISM
        } else if revision != Revision::St {
            side_tap(session, side, 4, false); // REV
            let count = [Revision::Ww, Revision::Ce, Revision::Hf, Revision::Super]
                .iter()
                .position(|&r| r == revision)
                .unwrap()
                + 1;
            for _ in 0..count {
                side_tap(session, side, 1, false);
            }
        }
        if loadout == Loadout::Old {
            assert_eq!(revision, Revision::St);
            side_tap(session, side, 8, false); // LOAD
            side_tap(session, side, 1, false);
        }
    }
    assert!(matches!(
        side_tap(session, side, 0, true),
        Some(Transition::Picked(_) | Transition::Fighters(_, _))
    ));
}

fn native_fight(styles: [Option<(Revision, Loadout)>; 2], training: bool) -> (Assets, Fight) {
    native_fight_with_config(styles, training, MatchConfig::default())
}

fn native_fight_with_config(
    styles: [Option<(Revision, Loadout)>; 2],
    training: bool,
    config: MatchConfig,
) -> (Assets, Fight) {
    let mut settings = Settings::default();
    settings.fill_meters = training;
    let mut session = GameSession::new(assets(), config).with_settings(settings);
    for _ in 0..200 {
        session.step(FrameInputs::default());
    }
    let mut input = FrameInputs::default();
    input.menu.start = true;
    assert_eq!(tap(&mut session, input), Some(Transition::OpenMainMenu));
    if training {
        choose(&mut session, MenuItem::SinglePlayer);
        choose(&mut session, MenuItem::Training);
        choose(&mut session, MenuItem::TrainingDummy);
    } else {
        choose(&mut session, MenuItem::MultiPlayer);
        choose(&mut session, MenuItem::Local);
    }
    select(&mut session, 0, styles[0]);
    select(&mut session, usize::from(!training), styles[1]);
    for _ in 0..40 {
        if session.screen() == "country select" {
            break;
        }
        session.step(FrameInputs::default());
    }
    assert_eq!(session.screen(), "country select");
    side_tap(&mut session, 0, 0, true);
    for _ in 0..1500 {
        if session.fight_machine().map(flow_position) == Some(FIGHT_LOOP) {
            let mut fight = session.fight().unwrap().clone();
            let assets = assets();
            for _ in 0..4 {
                fight.step(&assets, pad(0, false), pad(0, false), false);
            }
            return (assets, fight);
        }
        session.step(FrameInputs::default());
    }
    panic!("the native fight loop did not open");
}

#[test]
fn revision_damage_changes_the_native_health_and_st_meter_carries_to_round_two() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let mut remaining_health = Vec::new();
    let mut spawn_frames = Vec::new();
    for revision in [
        Revision::Ww,
        Revision::Ce,
        Revision::Hf,
        Revision::Super,
        Revision::St,
    ] {
        let (assets, mut fight) = native_fight([Some((revision, Loadout::Standard)), None], false);
        let [ryu, ken] = fight.machine().era_fighters();
        assert_eq!(ryu.style.revision, revision);
        assert_eq!((ryu.sub_hp, ken.sub_hp), (FULL_SUB_HP, FULL_SUB_HP));
        assert_eq!(ryu.gauge().is_some(), revision == Revision::St);
        for dir in [4, 5, 1] {
            fight.step(&assets, pad(dir, dir == 1), pad(0, false), false);
        }
        let mut spawned = None;
        for frame in 0..120 {
            fight.step(&assets, pad(0, false), pad(0, false), false);
            if spawned.is_none()
                && fight
                    .machine()
                    .toggle_state()
                    .era
                    .projectiles()
                    .iter()
                    .any(|p| p.hits_remaining != 0)
            {
                spawned = Some(frame);
            }
            if fight.machine().era_fighters()[1].sub_hp < FULL_SUB_HP {
                break;
            }
        }
        let [_, ken] = fight.machine().era_fighters();
        assert!(ken.sub_hp < FULL_SUB_HP, "{revision:?} Hadoken reaches Ken");
        spawn_frames.push(spawned.expect("native Hadoken spawned"));
        remaining_health.push(ken.sub_hp);
        let work = fight.machine().work_ram();
        let health = (P2_STRUCT - 0xff8000) as usize + 6;
        let native_points = u16::from(work[health + 1]) * 4 + u16::from(work[health]);
        assert_eq!(native_points, ken.sub_hp.div_ceil(20));
        if matches!(revision, Revision::Ww | Revision::Ce | Revision::Hf) {
            for _ in 0..120 {
                fight.step(&assets, pad(0, false), pad(0, false), false);
            }
            close_range(&mut fight, &assets);
            fight.step(&assets, pad(0, true), pad(0, false), false);
            let jab = fight.fight_read().fighters[0].move_id;
            for _ in 0..30 {
                if fight.machine().toggle_state().combo.hitstun[1] != 0 {
                    break;
                }
                fight.step(&assets, pad(0, false), pad(0, false), false);
            }
            let base = sf_engine::combo::move_stun(Some(TournamentFighter::Ryu), jab, 1).hit;
            assert!(base != 0, "{jab:#04x} must be a native ground attack");
            let expected = match revision {
                Revision::Ww if jab == 0x0d => 20,
                Revision::Ww => base + 2,
                Revision::Ce => base + 1,
                _ => base,
            };
            assert_eq!(
                fight.machine().toggle_state().combo.hitstun[1],
                expected,
                "{revision:?} jab stun"
            );
        }
        if revision == Revision::St {
            let meter = fight.machine().era_fighters()[0].super_meter;
            assert!((1..GAUGE_FULL).contains(&meter));
            let mut snapshot = fight.save_netplay_snapshot(0);
            let round = snapshot.machine.work[0x400a];
            snapshot.machine.work[0x0e18..0x0e1b].fill(0);
            fight.restore_netplay_snapshot(&snapshot);
            for _ in 0..1500 {
                fight.step(&assets, pad(0, false), pad(0, false), false);
                if fight.machine().work_ram()[0x400a] != round
                    && flow_position(fight.machine()) == FIGHT_LOOP
                {
                    break;
                }
            }
            assert_ne!(fight.machine().work_ram()[0x400a], round);
            assert_eq!(flow_position(fight.machine()), FIGHT_LOOP);
            assert_eq!(fight.machine().era_fighters()[0].super_meter, meter);
            assert_eq!(fight.machine().era_fighters()[1].sub_hp, FULL_SUB_HP);

            // Seed a live fractional final point, then finish it through
            // the normal pad and hit reaction path.
            let mut near_ko = fight.save_netplay_snapshot(0);
            near_ko.machine.toggle_state.era.fighters[1].sub_hp = 19;
            near_ko.machine.work[health] = 1;
            near_ko.machine.work[health + 1] = 0;
            fight.restore_netplay_snapshot(&near_ko);
            for _ in 0..16 {
                fight.step(&assets, pad(0, false), pad(0, false), false);
            }
            assert_eq!(fight.fight_read().fighters[1].health, 1);
            assert!(!fight.fight_read().fighters[1].knocked_out());
            for dir in [4, 5, 1] {
                fight.step(&assets, pad(dir, dir == 1), pad(0, false), false);
            }
            for _ in 0..120 {
                if fight.machine().era_fighters()[1].sub_hp == 0 {
                    break;
                }
                fight.step(&assets, pad(0, false), pad(0, false), false);
            }
            assert_eq!(fight.machine().era_fighters()[1].sub_hp, 0);
            assert_eq!(&fight.machine().work_ram()[health..health + 2], &[0, 0]);
            assert!(fight.fight_read().fighters[1].knocked_out());
        }
    }
    remaining_health.sort_unstable();
    remaining_health.dedup();
    assert_eq!(
        remaining_health.len(),
        5,
        "all five revisions adjust Hadoken damage"
    );
    assert!(
        spawn_frames[2] < spawn_frames[0],
        "Hyper Fighting starts its projectile before World Warrior"
    );
}

#[test]
fn training_full_meter_drives_a_native_super_freeze_and_refills_after_recovery() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let (assets, mut fight) = native_fight([Some((Revision::St, Loadout::Standard)), None], true);
    assert_eq!(fight.machine().era_fighters()[0].super_meter, GAUGE_FULL);
    for dir in [4, 5, 1, 4, 5] {
        fight.step(&assets, pad(dir, false), pad(0, false), false);
    }
    let activation = fight.step(&assets, pad(1, true), pad(0, false), false);
    assert!(activation.announcements.contains(&AnnouncerLine::Super));
    use sf_video::callouts::{CalloutKind, Side};
    let callouts = fight.callouts().frame();
    assert!(callouts.dim);
    assert_eq!(
        callouts.side(Side::P1).words[0],
        Some(CalloutKind::Super {
            name: "Shinku Hadoken"
        })
    );
    assert_eq!(fight.fight_read().fighters[0].move_id, SF2_SUPER);
    assert_eq!(fight.machine().era_super_freeze(), SUPER_FREEZE_FRAMES - 1);
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
    let mut without_callout = fight.clone();
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.callouts = sf_video::callouts::CalloutBoard::new();
    without_callout.restore_netplay_snapshot(&snapshot);
    let bright = without_callout.render(&assets);
    let quarter_dimmed = bright
        .argb
        .iter()
        .zip(&activation.frame.argb)
        .filter(|(base, shown)| {
            let [b, g, r, alpha] = base.to_le_bytes();
            let quarter = |channel: u8| (u16::from(channel) * 3 / 4) as u8;
            let expected = u32::from_le_bytes([quarter(b), quarter(g), quarter(r), alpha]);
            let half = u32::from_le_bytes([b / 2, g / 2, r / 2, alpha]);
            expected != **base && expected != half && expected == **shown
        })
        .count();
    assert!(quarter_dimmed >= sf_video::render::WIDTH);
    let frozen = fight.machine().state.objectram.clone();
    for left in (0..SUPER_FREEZE_FRAMES - 1).rev() {
        fight.step(&assets, pad(0, false), pad(2, true), false);
        assert_eq!(fight.machine().era_super_freeze(), left);
        assert_eq!(fight.machine().state.objectram, frozen);
    }
    for _ in 0..150 {
        fight.step(&assets, pad(0, false), pad(0, false), false);
        if fight.machine().era_fighters()[0].super_meter == GAUGE_FULL {
            break;
        }
    }
    assert_eq!(fight.machine().era_fighters()[0].super_meter, GAUGE_FULL);

    for _ in 0..60 {
        fight.step(&assets, pad(0, false), pad(0, false), false);
    }
    close_range(&mut fight, &assets);
    for dir in [4, 5, 1] {
        fight.step(&assets, pad(dir, false), pad(0, false), false);
    }
    let mut kick = pad(4, false);
    kick.buttons[3] = true;
    fight.step(&assets, kick, pad(0, false), false);
    for _ in 0..20 {
        if fight.machine().toggle_state().cancel.window[0] > 0 {
            break;
        }
        fight.step(&assets, pad(4, false), pad(0, false), false);
    }
    assert!(
        fight.machine().toggle_state().cancel.window[0] > 0,
        "crouching light kick opens a cancel at range {}, observed {:?}",
        center_x(&fight, P1_STRUCT).abs_diff(center_x(&fight, P2_STRUCT)),
        fight.fight_read()
    );
    let cancels = fight.machine().toggle_state().cancel.count[0];
    fight.step(&assets, pad(5, false), pad(0, false), false);
    let cancelled = fight.step(&assets, pad(1, true), pad(0, false), false);
    assert!(cancelled.announcements.contains(&AnnouncerLine::Super));
    assert_eq!(fight.machine().toggle_state().cancel.count[0], cancels + 1);

    let (_, old) = native_fight([Some((Revision::St, Loadout::Old)), None], true);
    assert_eq!(old.machine().era_fighters()[0].style.loadout, Loadout::Old);
    assert!(old.machine().era_fighters()[0].gauge().is_none());
}

#[test]
fn left_facing_supers_mirror_the_opening_grid_before_the_freeze() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }

    let grid = |fight: &Fight| {
        let machine = fight.machine();
        let entity = usize::from(machine.diagnostic_telemetry().p2.entity_offset);
        let obj = &machine.state.objectram;
        assert_ne!(obj[entity + 0x37] & 4, 0, "Ken faces left");
        let count = (usize::from(obj[entity + 0xc]) + 1) * (usize::from(obj[entity + 0xd]) + 1);
        (0..count)
            .map(|index| {
                let tile = entity + index * 0x40;
                (
                    u16::from_be_bytes([obj[tile], obj[tile + 1]]),
                    u16::from_be_bytes([obj[tile + 2], obj[tile + 3]]),
                )
            })
            .collect::<Vec<_>>()
    };

    let cases: [(Revision, Loadout, &[u8]); 2] = [
        (Revision::St, Loadout::Standard, &[4, 6, 2, 4, 6, 2]),
        (Revision::Sfz3, Loadout::AIsm, &[4, 6, 2, 4, 6]),
    ];
    let observed = cases.map(|(revision, loadout, motion)| {
        let (assets, mut fight) = native_fight([None, Some((revision, loadout))], false);
        let mut stocked = fight.save_netplay_snapshot(0);
        stocked.machine.toggle_state.era.fighters[1].super_meter = GAUGE_FULL;
        fight.restore_netplay_snapshot(&stocked);

        let (&last, prefix) = motion.split_last().unwrap();
        for &direction in prefix {
            fight.step(&assets, pad(0, false), pad(direction, false), false);
        }
        let activation = fight.step(&assets, pad(0, false), pad(last, true), false);
        assert!(activation.announcements.contains(&AnnouncerLine::Super));
        assert_eq!(fight.machine().era_super_freeze(), SUPER_FREEZE_FRAMES - 1);
        let opening = grid(&fight);
        for _ in 0..SUPER_FREEZE_FRAMES {
            fight.step(&assets, pad(0, false), pad(0, false), false);
        }
        let first_running = grid(&fight);
        (
            revision,
            opening.iter().all(|&(_, attr)| attr & 0x100 != 0),
            opening == first_running,
        )
    });
    assert_eq!(
        observed,
        [(Revision::St, true, true), (Revision::Sfz3, true, true)]
    );
}

#[test]
fn late_inputs_reproduce_sf2_super_freeze_meter_and_native_health() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let (assets, mut initial) =
        native_fight([None, Some((Revision::St, Loadout::Standard))], false);
    // Ken earns two points per Hadoken against the guarding Ryu.
    for _ in 0..GAUGE_FULL / 2 {
        for dir in [4, 6, 2] {
            initial.step(&assets, pad(2, false), pad(dir, dir == 2), false);
        }
        for _ in 0..120 {
            initial.step(&assets, pad(2, false), pad(0, false), false);
        }
        if initial.machine().era_fighters()[1].super_meter == GAUGE_FULL {
            break;
        }
    }
    assert_eq!(initial.machine().era_fighters()[1].super_meter, GAUGE_FULL);
    let mut reference = initial.clone();
    let mut predicted = initial;
    let mut net = RollbackEngine::new(0, 0, 8);
    let mut super_announced = false;
    const FRAMES: u32 = 100;
    const DELAY: u32 = 4;
    let remote = |frame: u32| {
        let dir = [4, 6, 2, 4, 6, 2].get(frame as usize).copied().unwrap_or(0);
        pad(dir, frame == 5)
    };
    for tick in 0..FRAMES + DELAY {
        if tick >= DELAY {
            let frame = tick - DELAY;
            net.apply_remote_inputs(
                &mut predicted,
                &assets,
                frame,
                &[encode_inputs(remote(frame))],
            );
        }
        if tick < FRAMES {
            let output = reference.step(&assets, pad(0, false), remote(tick), false);
            super_announced |= output.announcements.contains(&AnnouncerLine::Super);
            net.step(&mut predicted, &assets, pad(0, false))
                .expect("under the rollback limit");
        }
    }
    assert!(super_announced);
    assert_eq!(predicted.state_checksum(), reference.state_checksum());
    assert_eq!(
        predicted.machine().era_fighters(),
        reference.machine().era_fighters()
    );
    assert_eq!(
        predicted.machine().save_snapshot(),
        reference.machine().save_snapshot()
    );
}

fn center_x(fight: &Fight, ps: u32) -> u16 {
    let machine = fight.machine();
    let pointer = (ps - 0xff8000) as usize + 0x10;
    let work = machine.work_ram();
    let e = u16::from_be_bytes([work[pointer], work[pointer + 1]]) as usize;
    let obj = &machine.state.objectram;
    let sx = u16::from_be_bytes([obj[e + 6], obj[e + 7]]);
    let attr = u16::from_be_bytes([obj[e + 2], obj[e + 3]]);
    let width = if attr & 0x400 != 0 { 32 } else { 16 };
    sx.wrapping_add((u16::from(obj[e + 0xd]) + 1) * width / 2)
}

fn close_range(fight: &mut Fight, assets: &Assets) {
    for _ in 0..180 {
        if center_x(fight, P1_STRUCT).abs_diff(center_x(fight, P2_STRUCT)) <= 32 {
            break;
        }
        if fight.training() {
            fight.step(assets, pad(1, false), pad(0, false), false);
        } else {
            fight.step(assets, pad(0, false), pad(2, false), false);
        }
    }
    for _ in 0..16 {
        fight.step(assets, pad(0, false), pad(0, false), false);
    }
}

#[test]
fn sf2_throw_tech_follows_the_attacker_switch() {
    use sf_engine::combo::throw::THROWN_MOVE;
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    for untechable in [false, true] {
        let mut config = MatchConfig::default();
        config.sf1plus.untechable_sf2_throws = untechable;
        let (assets, mut fight) = native_fight_with_config(
            [Some((Revision::Ww, Loadout::Standard)), None],
            false,
            config,
        );
        close_range(&mut fight, &assets);
        let mut throw = pad(1, false);
        throw.buttons[2] = true;
        fight.step(&assets, throw, pad(0, false), false);
        assert_eq!(fight.fight_read().fighters[1].move_id, THROWN_MOVE);
        let mut tech = pad(0, true);
        tech.buttons[3] = true;
        fight.step(&assets, pad(0, false), tech, false);
        assert_eq!(
            fight.fight_read().fighters[1].move_id == THROWN_MOVE,
            untechable
        );
        assert_eq!(fight.machine().era_fighters()[1].sub_hp, FULL_SUB_HP);
    }
}

#[test]
fn air_tatsumaki_is_available_in_hyper_fighting_and_absent_in_world_warrior() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    for (revision, allowed) in [(Revision::Ww, false), (Revision::Hf, true)] {
        let (assets, mut fight) = native_fight([Some((revision, Loadout::Standard)), None], false);
        let idle = PlayerInputs::default();
        fight.step(&assets, pad(8, false), idle, false);
        for _ in 0..10 {
            fight.step(&assets, idle, idle, false);
        }
        for dir in [4, 6, 2] {
            let mut input = pad(dir, false);
            input.buttons[3] = dir == 2;
            fight.step(&assets, input, idle, false);
        }
        assert_eq!(
            fight.fight_read().fighters[0].move_id == 0x20,
            allowed,
            "{revision:?}"
        );
    }
}

#[test]
fn no_gauges_suppresses_st_super_even_with_fill_meters() {
    use sf_engine::era::GaugeMode;
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let mut config = MatchConfig::default();
    config.gauge_mode = GaugeMode::NoGauges;
    let (assets, mut fight) = native_fight_with_config(
        [Some((Revision::St, Loadout::Standard)), None],
        true,
        config,
    );
    let idle = PlayerInputs::default();
    assert!(fight.machine().era_fighters()[0].gauge().is_none());
    for (index, dir) in [4, 5, 1, 4, 5, 1].into_iter().enumerate() {
        let frame = fight.step(&assets, pad(dir, index == 5), idle, false);
        assert!(!frame.announcements.contains(&AnnouncerLine::Super));
    }
    assert_eq!(fight.machine().era_super_freeze(), 0);
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
}

#[test]
fn super_freeze_attack_buffer_respects_the_match_setting() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let style = Some((Revision::St, Loadout::Standard));
    let idle = PlayerInputs::default();
    for keep_inputs in [true, false] {
        let mut config = MatchConfig::default();
        config.sf1plus.disable_freeze_input_lockout = keep_inputs;
        let (assets, mut fight) = native_fight_with_config([style, style], false, config);
        let mut snapshot = fight.save_netplay_snapshot(0);
        snapshot.machine.toggle_state.era.fighters[0].super_meter = GAUGE_FULL;
        fight.restore_netplay_snapshot(&snapshot);
        for (index, dir) in [4, 5, 1, 4, 5, 1].into_iter().enumerate() {
            fight.step(&assets, pad(dir, index == 5), idle, false);
        }
        assert_eq!(fight.machine().era_super_freeze(), SUPER_FREEZE_FRAMES - 1);
        fight.step(&assets, idle, pad(0, true), false);
        assert!(!fight.fight_read().fighters[1].attacking());
        for _ in 0..SUPER_FREEZE_FRAMES {
            fight.step(&assets, idle, idle, false);
        }
        assert_eq!(fight.fight_read().fighters[1].attacking(), keep_inputs);
    }
}

#[test]
fn classic_special_damage_uses_the_original_hadoken_value() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let idle = PlayerInputs::default();
    for classic in [false, true] {
        let mut config = MatchConfig::default();
        config.sf1plus.classic_1987_special_damage = classic;
        let (assets, mut fight) = native_fight_with_config([None, None], false, config);
        for dir in [4, 5, 1] {
            fight.step(&assets, pad(dir, dir == 1), idle, false);
        }
        for _ in 0..120 {
            fight.step(&assets, idle, idle, false);
            if fight.machine().era_fighters()[1].sub_hp != FULL_SUB_HP {
                break;
            }
        }
        let expected = if classic { 20 } else { 9 };
        assert_eq!(
            fight.machine().era_fighters()[1].sub_hp,
            FULL_SUB_HP - expected * 20
        );
    }
}
