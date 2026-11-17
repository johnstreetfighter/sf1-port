//! Native SF2 commands from real Character Select and two-player pads.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod support;

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::data_bank::{SF2_SPECIAL_SLOT, SF2_SUPER_SLOT};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, PlayerInputs, Transition};
use sf_engine::front::settings::Settings;
use sf_engine::roster::TournamentFighter::{self, Ken, Ryu, Sagat};
use sf_engine::front::cabinet::flow_position;
use sf_video::art::{load_art_from, ArtSources};
use sf_video::art_pack::ArtPack;
use sf_video::callouts::Side;
use sf_video::romset::load_assets;
use sf_video::sprite_bank::{sprite_slot, ROM_SPRITE_TILES};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const PACK: &str = "../../assets/sf1-port-art";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const LP: usize = 0;
const MP: usize = 1;
const HP: usize = 2;
const LK: usize = 3;
const MK: usize = 4;
const HK: usize = 5;

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn pad(direction: u8, facing_right: bool, button: Option<usize>) -> PlayerInputs {
    let mut input = PlayerInputs::default();
    let forward = usize::from(!facing_right);
    let back = 1 - forward;
    input.directions[forward] = matches!(direction, 3 | 6 | 9);
    input.directions[back] = matches!(direction, 1 | 4 | 7);
    input.directions[2] = matches!(direction, 1..=3);
    input.directions[3] = matches!(direction, 7..=9);
    if let Some(button) = button {
        input.buttons[button] = true;
    }
    input
}

fn tap(session: &mut GameSession, input: FrameInputs) -> FrameOutput {
    let output = session.step(input);
    session.step(idle());
    output
}

fn p1(direction: u8, button: Option<usize>) -> FrameInputs {
    FrameInputs { p1: pad(direction, true, button), ..idle() }
}

fn p2(direction: u8, button: Option<usize>) -> FrameInputs {
    FrameInputs { p2: pad(direction, false, button), ..idle() }
}

fn select_st(session: &mut GameSession, confirm: FrameInputs, right: FrameInputs) {
    select_st_loadout(session, confirm, right, Loadout::Standard);
}

fn select_st_loadout(session: &mut GameSession, confirm: FrameInputs, right: FrameInputs, loadout: Loadout) {
    assert_eq!(tap(session, confirm).transition, Some(Transition::StyleOpened));
    assert_eq!(tap(session, right).transition, Some(Transition::CursorMoved));
    if loadout == Loadout::Old {
        tap(session, p1(8, None));
        tap(session, right);
    }
    tap(session, confirm);
}

fn until(session: &mut GameSession, ready: impl Fn(&GameSession) -> bool, label: &str) {
    for _ in 0..3000 {
        if ready(session) {
            return;
        }
        session.step(idle());
    }
    panic!("did not reach {label} from {}", session.screen());
}

fn fight(first: TournamentFighter, training: bool) -> Option<GameSession> {
    fight_with_loadout(first, training, Loadout::Standard)
}

fn fight_with_loadout(first: TournamentFighter, training: bool, loadout: Loadout) -> Option<GameSession> {
    if !Path::new(ROMS).is_dir() || (first == Sagat && !Path::new(PACK).is_dir()) {
        eprintln!("skipping: local ROM set or Sagat's art pack not present");
        return None;
    }
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let mut session = GameSession::new(assets, MatchConfig::definitive())
        .with_settings(Settings { fill_meters: training, ..Settings::default() });
    if first == Sagat {
        let pack = ArtPack::open(Path::new(PACK)).expect("the art pack opens");
        let art = load_art_from(
            session.assets(),
            &ArtSources {
                recipes: &[include_str!("../../../art/recipes/sagat.json")],
                reviews: Some(&[include_str!("../../../art/review/sagat.json")]),
                pack: Ok(&pack),
            },
        )
        .expect("Sagat's approved poses load");
        session = session.with_sprite_bank(&art.bank).with_player_tables(&art.catalogues);
    }

    for _ in 0..200 {
        session.step(idle());
    }
    let mut start = idle();
    start.menu.start = true;
    tap(&mut session, start);
    let confirm = p1(5, Some(LP));
    if training {
        tap(&mut session, confirm); // Single Player
        tap(&mut session, p1(2, None));
        tap(&mut session, confirm); // Training
        tap(&mut session, p1(2, None));
        assert_eq!(tap(&mut session, confirm).screen, "select"); // Dummy
    } else {
        tap(&mut session, p1(2, None));
        tap(&mut session, confirm); // Multi Player
        assert_eq!(tap(&mut session, confirm).screen, "select"); // Local
    }
    if first == Sagat {
        tap(&mut session, p1(2, None));
        for _ in 0..5 {
            tap(&mut session, p1(6, None));
        }
    } else if first == Ken {
        tap(&mut session, p1(6, None));
    }
    select_st_loadout(&mut session, confirm, p1(6, None), loadout);
    if training {
        tap(&mut session, confirm); // Ken
        tap(&mut session, confirm); // SF1+
    } else {
        select_st(&mut session, p2(5, Some(LP)), p2(4, None));
    }
    until(&mut session, |s| s.screen() == "country select", "country select");
    tap(&mut session, confirm); // Japan
    until(
        &mut session,
        |s| s.fight_machine().map(flow_position) == Some(FIGHT_LOOP),
        "fight loop",
    );
    for _ in 0..240 {
        session.step(idle());
    }
    let bout = session.fight().unwrap();
    assert_eq!([bout.player_fighter(0), bout.player_fighter(1)], [first, Ken]);
    let styles = bout.machine().era_fighters();
    assert_eq!(styles[0].style.revision, Revision::St);
    assert_eq!(styles[0].style.loadout, loadout);
    if !training {
        assert_eq!(styles[1].style.revision, Revision::St);
    }
    Some(session)
}

fn fighter_axis(fight: &sf_engine::front::fight::Fight, side: usize) -> (i32, usize) {
    let machine = fight.machine();
    let read = machine.diagnostic_telemetry();
    let fighter = [read.p1, read.p2][side];
    let entity = usize::from(fighter.entity_offset);
    let offset = machine.state.objectram[entity + 0x26] as i8;
    (fighter.center_x - i32::from(offset), entity)
}

fn world_axis(fight: &sf_engine::front::fight::Fight, side: usize) -> i32 {
    let camera = fight.machine().work_ram()[0x400c..0x400e].try_into().unwrap();
    i32::from(u16::from_be_bytes(camera)) + fighter_axis(fight, side).0
}

fn floor_height(fight: &sf_engine::front::fight::Fight, side: usize) -> i32 {
    let (_, entity) = fighter_axis(fight, side);
    let objects = &fight.machine().state.objectram;
    let attr = u16::from_be_bytes(objects[entity + 2..entity + 4].try_into().unwrap());
    let top = i32::from(u16::from_be_bytes(objects[entity + 4..entity + 6].try_into().unwrap()));
    let tile = if attr & 0x400 != 0 { 32 } else { 16 };
    sf_engine::data_bank::GROUND - top - tile * (i32::from(objects[entity + 0xc]) + 1)
}

fn position_sagat_fixture(fight: &mut sf_engine::front::fight::Fight, gap: i32) {
    position_sagat_fixture_at(fight, 145, gap);
}

fn position_sagat_fixture_at(fight: &mut sf_engine::front::fight::Fight, origin: i32, gap: i32) {
    let mut state = fight.save_netplay_snapshot(0);
    for (side, x) in [origin, origin + gap].into_iter().enumerate() {
        let (axis, entity) = fighter_axis(fight, side);
        let objects = &mut state.machine.state.objectram;
        let cells = (usize::from(objects[entity + 0xc]) + 1) * (usize::from(objects[entity + 0xd]) + 1);
        for cell in (entity..).step_by(0x40).take(cells) {
            let position = &mut objects[cell + 6..cell + 8];
            let moved = u16::from_be_bytes(position.try_into().unwrap()).wrapping_add((x - axis) as u16);
            position.copy_from_slice(&moved.to_be_bytes());
        }
    }
    fight.restore_netplay_snapshot(&state);
}

#[test]
fn st_sagat_walk_matches_world_turbo_two_elapsed_pacing() {
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some(session) = fight_with_loadout(Sagat, false, loadout) else { return };
        let assets = session.assets();
        for (dir, origin, gap, expected) in [(6, 135, 230, 105), (4, 265, 90, -82)] {
            let mut fight = session.fight().unwrap().clone();
            let neutral = pad(5, false, None);
            position_sagat_fixture_at(&mut fight, origin, gap);
            let start = world_axis(&fight, 0);
            let mut positions = vec![start];
            for _ in 0..32 {
                fight.step(assets, pad(dir, true, None), neutral, false);
                positions.push(world_axis(&fight, 0));
            }
            let case = format!("{loadout:?} direction {dir}");
            assert_eq!(positions[32] - start, expected, "{case}, positions {positions:?}");
            let forward = dir == 6;
            for limits in positions.windows(2) {
                let step = limits[1] - limits[0];
                assert!(if forward { matches!(step, 2 | 3 | 5) } else { matches!(step, -2 | -4) },
                    "{case}, step {step}");
            }
            for _ in 0..4 {
                fight.step(assets, pad(5, true, None), neutral, false);
                assert_eq!(world_axis(&fight, 0), positions[32], "{case}, neutral");
            }
            fight.step(assets, pad(dir, true, None), neutral, false);
            assert_eq!(world_axis(&fight, 0) - positions[32], positions[1] - start, "{case}, fresh start");
        }
    }
}

#[test]
fn st_sagat_tiger_shot_matches_world_releases_travel_and_postures() {
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some(session) = fight_with_loadout(Sagat, false, loadout) else { return };
        let assets = session.assets();
        for button in 0..6 {
            let low = button >= LK;
            let strength = button % 3;
            let release = if low && loadout == Loadout::Standard { 12 } else { 10 };
            let expected_ready = release + if low { 41 } else { 38 };
            for (gap, posture) in [(220, 5), (120, 5), (120, 4), (120, 2), (120, 1)] {
                let mut fight = session.fight().unwrap().clone();
                // The arcade schedules standing Back at display N+3.
                // Both measured phases first sample Back at action +4
                // (game ticks 20→24 or 21→25). Ken stays at the initial
                // axis gap until then.
                let setup_posture = if posture == 4 { 5 } else { posture };
                position_sagat_fixture(&mut fight, gap);
                for _ in 0..15 {
                    fight.step(assets, pad(5, true, None), pad(setup_posture, false, None), false);
                }
                position_sagat_fixture(&mut fight, gap);
                for direction in [2, 3] {
                    for _ in 0..3 {
                        fight.step(assets, pad(direction, true, None), pad(setup_posture, false, None), false);
                    }
                }
                fight.step(assets, pad(6, true, Some(button)), pad(setup_posture, false, None), false);
                let move_id = if low { 0x1b } else { 0x1a };
                assert_eq!(fight.fight_read().fighters[0].move_id, move_id);
                if gap == 220 {
                    assert_eq!(fighter_axis(&fight, 1).0 - fighter_axis(&fight, 0).0, gap);
                }
                let case = format!("{loadout:?}, button {button}, gap {gap}, posture {posture}");
                let expected_contact = match (gap, low, posture) {
                    (220, false, _) => Some((release + [31, 19, 13][strength], false)),
                    (220, true, _) => Some((release + [31, 19, 14][strength], false)),
                    (_, false, 1 | 2) => None,
                    (_, false, 4) | (_, true, 1) => Some((release, true)),
                    _ => Some((release, false)),
                };
                let mut spawn = None;
                let mut ready = None;
                let mut contact = None;
                let mut settled = false;
                let mut contact_axis = None;
                let mut pending = None;
                for tick in 1..=90 {
                    let defender_direction = if tick < 4 { setup_posture } else { posture };
                    fight.step(assets, pad(4, true, None), pad(defender_direction, false, None), false);
                    let read = fight.fight_read();
                    let machine = fight.machine();
                    let telemetry = machine.diagnostic_telemetry();
                    if telemetry.p1.spawned_projectile_this_frame {
                        spawn.get_or_insert(tick);
                    }
                    if gap == 220 && (release..=release + 3).contains(&tick) {
                        let shot = telemetry.projectiles.iter().find(|p| p.collision_flags & 0x80 != 0)
                            .expect("travelling Tiger Shot");
                        let e = usize::from(shot.slot_offset);
                        let objects = &machine.state.objectram;
                        let attr = u16::from_be_bytes(objects[e + 2..e + 4].try_into().unwrap());
                        let half = if attr & 0x400 != 0 { 16 } else { 8 };
                        let shot_x = i32::from(u16::from_be_bytes(objects[e + 6..e + 8].try_into().unwrap())) + half;
                        let shot_y = i32::from(u16::from_be_bytes(objects[e + 4..e + 6].try_into().unwrap())) + half;
                        let speed = [3, 5, 7][strength];
                        assert_eq!(shot_x - fighter_axis(&fight, 0).0, 104 + speed * (tick - release) as i32, "{case}");
                        assert_eq!(shot_y, sf_engine::data_bank::GROUND - if low { 38 } else { 78 }, "{case}");
                    }
                    if read.connected(Side::P1) {
                        assert!(contact.is_none(), "{case}: each Tiger Shot can contact only once");
                        contact = Some((tick, read.blocked[0]));
                        contact_axis = Some(world_axis(&fight, 1));
                        let snapshot = fight.save_netplay_snapshot(tick);
                        assert_eq!(snapshot.machine.toggle_state.combo.contact_freeze, [0, 14, 0], "{case}");
                        if loadout == Loadout::Old && button == LK && posture == 1 && gap == 120 {
                            pending = Some(snapshot);
                        }
                        let hp = match (low, read.blocked[0]) {
                            (false, false) => [16, 17, 19][strength],
                            (true, false) => [15, 16, 17][strength],
                            (false, true) => 4,
                            (true, true) => [3, 4, 4][strength],
                        };
                        let full_hp = sf_engine::era_fight::FULL_SUB_HP;
                        assert_eq!(full_hp - machine.era_fighters()[1].sub_hp, (hp * full_hp + 72) / 144, "{case}");
                    }
                    if read.fighters[0].move_id == 3 {
                        ready.get_or_insert(tick);
                    }
                    if let Some((when, blocked)) = contact {
                        let clock = tick - when;
                        if clock < 36 {
                            let defender = read.fighters[1];
                            assert!(if blocked { defender.blockstun } else { defender.hitstun() }, "{case}, contact +{clock}");
                            if clock == 35 && gap == 120 {
                                // Recoil ends before the first actionable Back step.
                                let recoil = world_axis(&fight, 1) - contact_axis.unwrap();
                                assert_eq!(recoil, 78, "{case}");
                            }
                        } else if clock == 36 {
                            assert!(!read.fighters[1].locked() && !read.fighters[1].hitstun(), "{case}");
                            settled = true;
                        }
                    }
                }
                assert_eq!(spawn, Some(release), "{case}");
                assert_eq!(ready, Some(expected_ready), "{case}");
                assert_eq!(contact, expected_contact, "{case}");
                assert_eq!(settled, contact.is_some(), "{case}");
                if let Some(snapshot) = pending {
                    let checksum = fight.state_checksum();
                    fight.restore_netplay_snapshot(&snapshot);
                    for _ in release + 1..=90 {
                        fight.step(assets, pad(4, true, None), pad(posture, false, None), false);
                    }
                    assert_eq!(fight.state_checksum(), checksum, "{case}: rollback");
                }
            }
        }
    }
}

#[test]
fn st_sagat_knee_and_uppercut_match_world_whiff_and_close_routes() {
    let source = include_str!("../../../docs/research/sagat-super-turbo-melee-whiff-trajectory.tsv");
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some(session) = fight_with_loadout(Sagat, false, loadout) else { return };
        let assets = session.assets();
        for (button, name, ready, active) in [
            (0, "lp", 51, 3..=23),
            (1, "mp", 61, 3..=31),
            (2, "hp", 71, 3..=35),
            (3, "lk", 35, 4..=14),
            (4, "mk", 37, 4..=15),
            (5, "hk", 40, 5..=17),
        ] {
            let knee = button >= LK;
            let directions = if knee { [2, 6, 9] } else { [6, 2, 3] };
            let move_id = if knee { 0x19 } else { SF2_SPECIAL_SLOT };
            let trajectory: Vec<_> = source.lines().skip(1).filter_map(|line| {
                let mut fields = line.split('\t');
                (fields.next()? == name).then(|| {
                    let tick = fields.next().unwrap().parse::<usize>().unwrap();
                    let x = fields.next().unwrap().parse::<f64>().unwrap().round() as i32;
                    let height = fields.next().unwrap().parse::<i32>().unwrap();
                    (tick, x, height)
                })
            }).collect();
            for (gap, posture) in [(220, 5), (42, 5), (42, 4), (42, 1)] {
                let mut fight = session.fight().unwrap().clone();
                let defender = pad(posture, false, None);
                for _ in 0..15 {
                    fight.step(assets, pad(5, true, None), defender, false);
                }
                position_sagat_fixture(&mut fight, gap);
                for &direction in &directions[..2] {
                    for _ in 0..2 {
                        fight.step(assets, pad(direction, true, None), defender, false);
                    }
                }
                if gap == 220 {
                    position_sagat_fixture(&mut fight, gap);
                }
                fight.step(assets, pad(directions[2], true, Some(button)), defender, false);
                let case = format!("{loadout:?} {name} gap {gap} posture {posture}");
                assert_eq!(fight.fight_read().fighters[0].move_id, move_id, "{case}");
                let origin = world_axis(&fight, 0);
                let mut contacts = Vec::new();
                let mut exit = None;
                let mut saved_launch = None;
                for tick in 1..=150 {
                    fight.step(assets, pad(5, true, None), defender, false);
                    let read = fight.fight_read();
                    if read.connected(Side::P1) {
                        contacts.push(tick);
                        assert_eq!(read.blocked[0], posture != 5, "{case}, tick {tick}");
                        let snapshot = fight.save_netplay_snapshot(tick as u32);
                        assert_eq!(snapshot.machine.toggle_state.combo.contact_freeze, [14, 14, 0], "{case}, tick {tick}");
                        if gap == 42 && name == "hp" && loadout == Loadout::Standard && posture == 5 {
                            assert_eq!(read.fighters[1].state & 3 == 1, tick != 3, "{case}, tick {tick}");
                            if tick == 21 {
                                saved_launch = Some(snapshot);
                            }
                        }
                    }
                    if !read.fighters[0].attacking() {
                        exit = Some(tick);
                        break;
                    }
                    if gap == 220 {
                        assert_eq!(read.fighters[0].attack_active, active.contains(&tick), "{case}, active {tick}");
                        let (_, x, height) = trajectory[tick];
                        assert_eq!(world_axis(&fight, 0) - origin, x, "{case}, X tick {tick}");
                        assert_eq!(floor_height(&fight, 0), height, "{case}, Y tick {tick}");
                    }
                }
                if gap == 220 {
                    assert!(contacts.is_empty(), "{case}: {contacts:?}");
                    assert_eq!(exit, Some(ready), "{case}");
                } else {
                    let expected: &[usize] = match (knee, button, loadout, posture) {
                        (true, 5, _, 1) => &[5],
                        (true, _, _, 1) => &[4],
                        (true, 5, _, _) => &[5, 25],
                        (true, _, _, _) => &[4, 24],
                        (false, 2, Loadout::Standard, 5) => &[3, 21, 38, 56, 74],
                        (false, 2, Loadout::Standard, 4) => &[3, 21, 38],
                        (false, 2, Loadout::Standard, 1) => &[3, 21],
                        _ => &[3],
                    };
                    assert_eq!(contacts, expected, "{case}");
                    assert_eq!(exit, Some(ready + expected.len() * 14), "{case}");
                }
                if let Some(snapshot) = saved_launch {
                    let checksum = fight.state_checksum();
                    fight.restore_netplay_snapshot(&snapshot);
                    for _ in 22..=exit.unwrap() {
                        fight.step(assets, pad(5, true, None), defender, false);
                    }
                    assert_eq!(fight.state_checksum(), checksum, "{case}: rollback");
                }
            }
        }
    }
}

#[test]
fn st_sagat_tiger_genocide_replays_the_world_flash_and_contacts() {
    const GAUGE_FULL: u8 = 48;
    let Some(session) = fight(Sagat, false) else { return };
    let assets = session.assets();
    let trajectory: Vec<_> = include_str!("../../../docs/research/sagat-super-turbo-genocide-whiff-trajectory.tsv")
        .lines().skip(1).map(|line| {
            let mut fields = line.split('\t').skip(1);
            let x = fields.next().unwrap().parse::<f64>().unwrap().floor() as i32;
            let y = fields.next().unwrap().parse::<i32>().unwrap();
            (x, y)
        }).collect();
    for (posture, airborne_whiff) in [(5, true), (5, false), (4, false), (1, false)] {
        let mut fight = session.fight().unwrap().clone();
        let defender = pad(posture, false, None);
        for _ in 0..15 {
            fight.step(assets, pad(5, true, None), defender, false);
        }
        let mut stock = fight.save_netplay_snapshot(0);
        stock.machine.toggle_state.era.fighters[0].super_meter = GAUGE_FULL;
        fight.restore_netplay_snapshot(&stock);
        for direction in [2, 3, 6, 2, 3] {
            fight.step(assets, pad(direction, true, None), defender, false);
        }
        position_sagat_fixture(&mut fight, if airborne_whiff { 220 } else { 40 });
        fight.step(assets, pad(6, true, Some(LP)), defender, false);
        assert_eq!(fight.fight_read().fighters[0].move_id, SF2_SUPER_SLOT);
        assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
        // The entry tick is the first of the 18 flash ticks.
        assert_eq!(fight.machine().era_super_freeze(), 17);
        let start_x = world_axis(&fight, 0);
        let mut contacts = Vec::new();
        let mut ready = None;
        let mut replay = None;
        for tick in 1..=170 {
            if airborne_whiff {
                // The arcade whiff fixture holds Ken outside Sagat's boxes.
                let mut high = fight.save_netplay_snapshot(tick as u32);
                let (_, e) = fighter_axis(&fight, 1);
                let objects = &mut high.machine.state.objectram;
                let cells = (usize::from(objects[e + 0xc]) + 1) * (usize::from(objects[e + 0xd]) + 1);
                for cell in (e..).step_by(0x40).take(cells) {
                    objects[cell + 4..cell + 6].copy_from_slice(&0x0050_u16.to_be_bytes());
                }
                objects[e + 0xf] |= 8;
                fight.restore_netplay_snapshot(&high);
            }
            let flash = fight.machine().era_super_freeze();
            fight.step(assets, pad(5, true, None), defender, false);
            let read = fight.fight_read();
            assert_eq!(flash, 18_u8.saturating_sub(tick as u8));
            if read.connected(Side::P1) {
                let snapshot = fight.save_netplay_snapshot(tick as u32);
                assert_eq!(snapshot.machine.toggle_state.combo.contact_freeze, [6, 8, 0], "posture {posture}, tick {tick}");
                contacts.push((tick, read.blocked[0]));
                if !airborne_whiff && posture == 5 {
                    assert_eq!(read.fighters[1].state & 3 == 1, tick >= 42);
                    if tick == 42 {
                        replay = Some(snapshot);
                    }
                }
            }
            if !read.fighters[0].attacking() {
                ready = Some(tick);
                break;
            }
            if airborne_whiff && tick < 118 {
                let (x, y) = trajectory[tick];
                assert_eq!(world_axis(&fight, 0) - start_x, x, "whiff X tick {tick}, flash {flash}");
                assert_eq!(floor_height(&fight, 0), y, "whiff Y tick {tick}");
                let active = [24..=26, 27..=29, 30..=35, 52..=55, 56..=58, 59..=85]
                    .into_iter().any(|range| range.contains(&tick));
                assert_eq!(read.fighters[0].attack_active, active, "whiff strike tick {tick}");
            }
        }
        let expected: &[(usize, bool)] = match (airborne_whiff, posture) {
            (true, _) => &[],
            (false, 5) => &[(24, false), (33, false), (42, false), (74, false), (83, false)],
            (false, 4) => &[(24, true), (33, true), (42, true), (70, true), (80, true), (89, true)],
            _ => &[(24, true), (33, true)],
        };
        assert_eq!(contacts, expected, "posture {posture}");
        assert_eq!(ready, Some(118 + 6 * expected.len()), "posture {posture}");
        if let Some(snapshot) = replay {
            let checksum = fight.state_checksum();
            fight.restore_netplay_snapshot(&snapshot);
            for _ in 43..=148 {
                fight.step(assets, pad(5, true, None), defender, false);
            }
            assert_eq!(fight.state_checksum(), checksum, "Genocide juggle rollback");
        }
    }
}

#[test]
fn st_sagat_tiger_genocide_uses_both_world_motions_and_the_stock_gate() {
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some(session) = fight_with_loadout(Sagat, false, loadout) else { return };
        let assets = session.assets();
        let idle = pad(5, true, None);
        for (stock, short, button, expected) in [
            (48, false, LP, if loadout == Loadout::Standard { SF2_SUPER_SLOT } else { SF2_SPECIAL_SLOT }),
            (48, false, LK, 0x1b),
            (48, true, LK, if loadout == Loadout::Standard { SF2_SUPER_SLOT } else { 0x1b }),
            (47, false, LP, SF2_SPECIAL_SLOT),
            (49, false, LP, SF2_SPECIAL_SLOT),
        ] {
            let mut fight = session.fight().unwrap().clone();
            let mut snapshot = fight.save_netplay_snapshot(0);
            snapshot.machine.toggle_state.era.fighters[0].super_meter = stock;
            fight.restore_netplay_snapshot(&snapshot);
            let motion: &[u8] = if short { &[2, 3, 6, 2] } else { &[2, 3, 6, 2, 3] };
            for &direction in motion {
                fight.step(assets, pad(direction, true, None), idle, false);
            }
            fight.step(assets, pad(if short { 3 } else { 6 }, true, Some(button)), idle, false);
            let actual = fight.fight_read().fighters[0].move_id;
            assert_eq!(actual, expected, "{loadout:?} stock {stock} short {short} button {button}");
            assert_eq!(fight.machine().era_super_freeze(), if expected == SF2_SUPER_SLOT { 17 } else { 0 });
            if expected == SF2_SUPER_SLOT {
                assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
            }
        }
    }
}

#[test]
fn st_sagat_crouching_normals_match_the_world_clocks() {
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some(session) = fight_with_loadout(Sagat, false, loadout) else { return };
        let assets = session.assets();
        let old = loadout == Loadout::Old;
        for (button, startup, active_end, ready, stun, push, damage) in [
            (LP, 5, 7, 12, 27, 30, 11),
            (LK, 5, 8, 14, 27, 30, if old { 11 } else { 4 }),
            (MP, 5, 8, 15, 32, 54, 19),
            (MK, 5, 8, 16, 32, 54, 19),
            (HP, 7, 12, 26, 36, 78, if old { 26 } else { 18 }),
            (HK, 7, 12, 30, 36, 78, if old { 27 } else { 18 }),
        ] {
            for (gap, posture) in [(150, 5), (38, 5), (38, 1), (38, 4)] {
                let mut fight = session.fight().unwrap().clone();
                let ken = pad(posture, false, None);
                for _ in 0..15 {
                    fight.step(assets, pad(2, true, None), ken, false);
                }
                position_sagat_fixture(&mut fight, gap);
                let full_hp = fight.machine().era_fighters()[1].sub_hp;
                fight.step(assets, pad(2, true, Some(button)), ken, false);
                let mut contact_axis = None;
                let whiff = gap == 150;
                let guard = posture == 1 || (posture == 4 && !matches!(button, LK | MK | HK));
                let sweep = button == HK && !guard && !whiff;
                let reaction_end = if sweep { 95 } else { stun };
                let case = format!("{loadout:?} {button} gap {gap} posture {posture}");
                for tick in 1..=startup + reaction_end {
                    fight.step(assets, pad(2, true, None), ken, false);
                    let read = fight.fight_read();
                    assert_eq!(read.fighters[0].attacking(), tick < ready + if whiff { 0 } else { 14 }, "{case}, tick {tick}");
                    if whiff {
                        assert_eq!(read.fighters[0].attack_active, (startup..=active_end).contains(&tick), "{case}, tick {tick}");
                        continue;
                    }
                    assert_eq!(read.connected(Side::P1), tick == startup, "{case}, tick {tick}");
                    if tick == startup {
                        assert_eq!(read.blocked[0], guard, "{case}");
                        let freeze = fight.save_netplay_snapshot(0).machine.toggle_state.combo.contact_freeze;
                        assert_eq!(freeze, [14, 14, 0], "{case}");
                        let lost = full_hp - fight.machine().era_fighters()[1].sub_hp;
                        assert_eq!(lost, if guard { 0 } else { (damage * full_hp + 72) / 144 }, "{case}");
                        contact_axis = Some(world_axis(&fight, 1));
                    }
                    let defender = read.fighters[1];
                    let reacting = if sweep { defender.reacting() } else { defender.hitstun() || defender.blockstun };
                    assert_eq!(reacting, (startup..startup + reaction_end).contains(&tick), "{case}, tick {tick}");
                    if tick == startup + reaction_end - 1 {
                        let displacement = world_axis(&fight, 1) - contact_axis.unwrap();
                        assert_eq!(displacement, if sweep { 80 } else { push }, "{case}");
                    }
                    if sweep && tick >= startup {
                        let age: i32 = tick - startup;
                        let travel = 2 * (age - 16).clamp(0, 40);
                        assert_eq!(world_axis(&fight, 1) - contact_axis.unwrap(), travel, "{case}, tick {tick}");
                        let height = match age {
                            0..=16 | 56.. => Some(0),
                            17 => Some(5),
                            34..=38 => Some(55),
                            55 => Some(4),
                            _ => None,
                        };
                        if let Some(height) = height {
                            assert_eq!(floor_height(&fight, 1), height, "{case}, tick {tick}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn st_sagat_standing_normals_match_world_proximity_and_strike_windows() {
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some(session) = fight_with_loadout(Sagat, false, loadout) else { return };
        let assets = session.assets();
        let ken = pad(5, false, None);
        let old = loadout == Loadout::Old;
        for (button, gap, startup, active_end, ready) in [
            (LP, 81, 4, 7, 12), (LP, 82, 4, 7, 13),
            (MP, 68, if old { 5 } else { 4 }, if old { 10 } else { 9 }, if old { 18 } else { 17 }),
            (MP, 69, 6, 9, 19),
            (HP, 92, if old { 7 } else { 4 }, if old { 14 } else { 11 }, if old { 26 } else { 23 }),
            (HP, 93, 8, 13, 37),
            (LK, 40, 4, 7, 15), (MK, 40, 4, 7, 21), (HK, 40, 4, 9, 33),
            (LK, 140, 4, 7, 15), (MK, 140, 4, 7, 21), (HK, 140, 4, 9, 33),
        ] {
            let mut fight = session.fight().unwrap().clone();
            position_sagat_fixture(&mut fight, gap);
            fight.step(assets, pad(5, true, Some(button)), ken, false);
            let case = format!("{loadout:?} button {button} gap {gap}");
            assert!(fight.fight_read().fighters[0].attacking(), "{case}");
            for tick in 1..=ready {
                let mut snapshot = fight.save_netplay_snapshot(tick);
                let (_, entity) = fighter_axis(&fight, 1);
                snapshot.machine.state.objectram[entity + 0xf] |= 8;
                fight.restore_netplay_snapshot(&snapshot);
                fight.step(assets, pad(5, true, None), ken, false);
                let read = fight.fight_read();
                assert_eq!(read.fighters[0].attacking(), tick < ready, "{case}, tick {tick}");
                assert_eq!(read.fighters[0].attack_active, (startup..=active_end).contains(&tick), "{case}, tick {tick}");
                assert!(!read.connected(Side::P1), "{case}, tick {tick}");
            }
        }
    }
}

#[test]
fn st_sagat_standing_punches_match_world_contact_and_recoil() {
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some(session) = fight_with_loadout(Sagat, false, loadout) else { return };
        let assets = session.assets();
        let old = loadout == Loadout::Old;
        for (button, gap, startup, stun, push, damage, crouch_reaches) in [
            (LP, 40, 4, 27, 30, 11, true),
            (MP, 40, if old { 5 } else { 4 }, 32, 54, 19, true),
            (HP, 40, if old { 7 } else { 4 }, 36, 78, if old { 24 } else { 23 }, false),
            (LP, 82, 4, 27, 30, 11, false),
            (MP, 69, 6, 32, 54, if old { 22 } else { 19 }, false),
            (HP, 93, 8, 36, 78, if old { 27 } else { 21 }, false),
        ] {
            for posture in [5, 4, 1] {
                let mut fight = session.fight().unwrap().clone();
                let ken = pad(posture, false, None);
                for _ in 0..15 {
                    fight.step(assets, pad(5, true, None), ken, false);
                }
                position_sagat_fixture(&mut fight, gap);
                let full_hp = fight.machine().era_fighters()[1].sub_hp;
                fight.step(assets, pad(5, true, Some(button)), ken, false);
                let blocked = posture != 5;
                let reaches = posture != 1 || crouch_reaches;
                let mut contact_axis = None;
                let case = format!("{loadout:?} button {button} gap {gap} posture {posture}");
                for tick in 1..=startup + stun {
                    fight.step(assets, pad(5, true, None), ken, false);
                    let read = fight.fight_read();
                    assert_eq!(read.connected(Side::P1), reaches && tick == startup, "{case}, tick {tick}");
                    if reaches && tick == startup {
                        assert_eq!(read.blocked[0], blocked, "{case}");
                        let freeze = fight.save_netplay_snapshot(0).machine.toggle_state.combo.contact_freeze;
                        assert_eq!(freeze, [14, 14, 0], "{case}");
                        contact_axis = Some(world_axis(&fight, 1));
                    }
                    let reacting = read.fighters[1].hitstun() || read.fighters[1].blockstun;
                    assert_eq!(reacting, reaches && (startup..startup + stun).contains(&tick), "{case}, tick {tick}");
                    if reaches && tick == startup + stun - 1 {
                        assert_eq!(world_axis(&fight, 1) - contact_axis.unwrap(), push, "{case}");
                    }
                }
                let expected = if blocked { 0 } else { (damage * full_hp + 72) / 144 };
                assert_eq!(full_hp - fight.machine().era_fighters()[1].sub_hp, expected, "{case}");
            }
        }
    }
}

#[test]
fn st_sagat_standing_kicks_replay_the_world_contact_sequences() {
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some(session) = fight_with_loadout(Sagat, false, loadout) else { return };
        let assets = session.assets();
        let old = loadout == Loadout::Old;
        for (gap, button, posture, expected, guard, stun, push, damage) in [
            (40, LK, 5, &[4, 20][..], false, 26, 34, 22),
            (40, LK, 4, &[4, 20][..], false, 26, 34, 22),
            (40, LK, 1, &[4, 20][..], true, 26, 34, 0),
            (40, MK, 5, &[4, 20][..], false, 31, 60, if old { 26 } else { 28 }),
            (40, MK, 4, &[4, 20][..], true, 31, 60, 0),
            (40, MK, 1, &[4][..], true, 32, 54, 0),
            (40, HK, 5, &[4][..], false, 36, 78, 14),
            (40, HK, 4, &[4][..], true, 36, 78, 0),
            (40, HK, 1, &[4][..], true, 36, 78, 0),
            (100, LK, 5, &[6][..], false, 27, 30, 11),
            (100, LK, 4, &[6][..], true, 27, 30, 0),
            (100, LK, 1, &[6][..], true, 27, 30, 0),
            (100, MK, 5, &[6][..], false, 32, 54, if old { 18 } else { 16 }),
            (100, MK, 4, &[6][..], true, 32, 54, 0),
            (100, MK, 1, &[][..], false, 0, 0, 0),
            (100, HK, 5, &[7][..], false, 36, 78, if old { 19 } else { 20 }),
            (100, HK, 4, &[7][..], true, 36, 78, 0),
            (100, HK, 1, &[][..], false, 0, 0, 0),
        ] {
            let mut fight = session.fight().unwrap().clone();
            let ken = pad(posture, false, None);
            for _ in 0..15 {
                fight.step(assets, pad(5, true, None), ken, false);
            }
            position_sagat_fixture(&mut fight, gap);
            let full_hp = fight.machine().era_fighters()[1].sub_hp;
            fight.step(assets, pad(5, true, Some(button)), ken, false);
            let mut contacts = Vec::new();
            let mut axis_at_contact = 0;
            let mut recovered = None;
            let case = format!("{loadout:?} button {button} posture {posture} gap {gap}");
            for tick in 1..=85 {
                fight.step(assets, pad(5, true, None), ken, false);
                let read = fight.fight_read();
                if read.connected(Side::P1) {
                    if contacts.is_empty() {
                        axis_at_contact = world_axis(&fight, 1);
                    }
                    contacts.push(tick);
                    assert_eq!(read.blocked[0], guard, "{case}, tick {tick}");
                    let freeze = fight.save_netplay_snapshot(0).machine.toggle_state.combo.contact_freeze;
                    assert_eq!(freeze, [14, 14, 0], "{case}, tick {tick}");
                    recovered = None;
                }
                if !contacts.is_empty() && !read.fighters[1].hitstun() && !read.fighters[1].blockstun {
                    recovered.get_or_insert(tick);
                }
                if expected.last().is_some_and(|last| tick + 1 == last + stun) {
                    assert_eq!(world_axis(&fight, 1) - axis_at_contact, push, "{case}");
                }
            }
            assert_eq!(contacts, expected, "{case}");
            assert_eq!(recovered, expected.last().map(|tick| tick + stun), "{case}");
            let expected_sub_hp = match damage {
                22 => 2 * ((11 * full_hp + 72) / 144),
                26 => (11 * full_hp + 72) / 144 + (15 * full_hp + 72) / 144,
                28 => (12 * full_hp + 72) / 144 + (16 * full_hp + 72) / 144,
                _ => (damage * full_hp + 72) / 144,
            };
            assert_eq!(full_hp - fight.machine().era_fighters()[1].sub_hp, expected_sub_hp, "{case}");
        }
    }
}

#[test]
fn st_sagat_aerial_normals_preserve_the_world_jump_arc_and_hit_windows() {
    let reference: Vec<_> = include_str!("../../../docs/research/sagat-super-turbo-jump-trajectory.tsv")
        .lines().skip(1).map(|line| {
            let mut values = line.split('\t');
            let loadout = values.next().unwrap();
            let direction = values.next().unwrap();
            let tick = values.next().unwrap().parse::<u32>().unwrap();
            let x = values.next().unwrap().parse::<i32>().unwrap() >> 16;
            let height = values.next().unwrap().parse::<i32>().unwrap();
            (loadout, direction, tick, x, height)
        }).collect();
    let controls = [LP, MP, HP, LK, MK, HK].map(Some);
    for (loadout, label) in [(Loadout::Standard, "new"), (Loadout::Old, "old")] {
        let Some(session) = fight_with_loadout(Sagat, false, loadout) else { return };
        let assets = session.assets();
        let idle = pad(5, false, None);
        for (direction, name, new_land) in [(8, "n", 46), (9, "f", 45), (7, "b", 48)] {
            let land = new_land - if loadout == Loadout::Old { 2 } else { 0 };
            for button in controls.into_iter().chain([None]) {
                let mut fight = session.fight().unwrap().clone();
                position_sagat_fixture_at(&mut fight, if direction == 7 { 240 } else { 145 }, 140);
                let origin = world_axis(&fight, 0);
                let case = format!("{label} direction {name} button {button:?}");
                let active = match button {
                    Some(LP) if direction == 8 => 14..land,
                    Some(LP) => 14..38,
                    Some(MP) => 14..28,
                    Some(HP) if direction == 8 => 16..26,
                    Some(HP | HK) => 16..24,
                    Some(LK) => 13..land,
                    Some(MK) => 14..34,
                    _ => 0..0,
                };
                for &(_, _, tick, dx, height) in reference.iter().filter(|row| row.0 == label && row.1 == name) {
                    let mut high = fight.save_netplay_snapshot(tick);
                    let (_, entity) = fighter_axis(&fight, 1);
                    high.machine.state.objectram[entity + 0xf] |= 8;
                    fight.restore_netplay_snapshot(&high);
                    let input = pad(if tick == 0 { direction } else { 5 }, true, if tick == 11 { button } else { None });
                    fight.step(assets, input, idle, false);
                    let read = fight.fight_read().fighters[0];
                    assert_eq!(world_axis(&fight, 0) - origin, dx, "{case}, X tick {tick}");
                    assert_eq!(floor_height(&fight, 0), height, "{case}, height tick {tick}");
                    assert_eq!(read.attack_active, active.contains(&tick), "{case}, active tick {tick}");
                    assert_eq!(read.move_id == 1, tick == land + 7, "{case}, ready tick {tick}");
                }
            }
        }
    }
}

#[test]
fn st_sagat_directional_mk_replays_the_late_jump_in_contacts() {
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some(session) = fight_with_loadout(Sagat, false, loadout) else { return };
        let assets = session.assets();
        let old = loadout == Loadout::Old;
        for (posture, dispatch, contact, blocked, stun, damage) in [
            (5, if old { 28 } else { 27 }, Some(if old { 38 } else { 40 }), false, 27, if old { 19 } else { 21 }),
            (4, if old { 28 } else { 27 }, Some(if old { 38 } else { 40 }), true, 32, 0),
            (2, if old { 28 } else { 27 }, if old { None } else { Some(44) }, false, 32, 19),
            (1, 28, if old { None } else { Some(44) }, false, 32, 18),
        ] {
            let mut fight = session.fight().unwrap().clone();
            let ken = pad(if posture == 4 { 5 } else { posture }, false, None);
            for _ in 0..15 {
                fight.step(assets, pad(5, true, None), ken, false);
            }
            position_sagat_fixture(&mut fight, 105);
            let hp = fight.machine().era_fighters()[1].sub_hp;
            let case = format!("{loadout:?}, posture {posture}");
            let mut contacts = Vec::new();
            let mut ready = None;
            for tick in 0..=80 {
                let button = (tick == dispatch).then_some(MK);
                let input = pad(if tick == 0 { 9 } else { 5 }, true, button);
                let defender = if posture == 4 && tick >= dispatch { pad(4, false, None) } else { ken };
                fight.step(assets, input, defender, false);
                let read = fight.fight_read();
                if read.connected(Side::P1) {
                    contacts.push(tick);
                    assert_eq!(read.blocked[0], blocked, "{case}");
                    let freeze = fight.save_netplay_snapshot(tick).machine.toggle_state.combo.contact_freeze;
                    assert_eq!(freeze, [14, 14, 0], "{case}");
                }
                let reacting = read.fighters[1].hitstun() || read.fighters[1].blockstun;
                assert_eq!(reacting, contact.is_some_and(|at| (at..at + stun).contains(&tick)), "{case}, tick {tick}");
                if read.fighters[0].move_id == 1 {
                    ready.get_or_insert(tick);
                }
            }
            assert_eq!(contacts.first().copied(), contact, "{case}");
            assert_eq!(contacts.len(), usize::from(contact.is_some()), "{case}");
            let expected_hp = if contact.is_some() { (damage * hp + 72) / 144 } else { 0 };
            assert_eq!(hp - fight.machine().era_fighters()[1].sub_hp, expected_hp, "{case}");
            assert_eq!(ready, Some(52 - u32::from(old) * 2 + u32::from(contact.is_some()) * 14), "{case}");
        }
    }
}

#[test]
fn st_sagat_tiger_genocide_cancels_a_crouching_jab_before_hitstop_ends() {
    let Some(session) = fight(Sagat, false) else { return };
    let assets = session.assets();
    for (finish, guard) in [(7, false), (8, false), (8, true)] {
        let ken = pad(if guard { 1 } else { 5 }, false, None);
        let mut fight = session.fight().unwrap().clone();
        let mut snapshot = fight.save_netplay_snapshot(0);
        snapshot.machine.toggle_state.era.fighters[0].super_meter = 48;
        fight.restore_netplay_snapshot(&snapshot);
        for _ in 0..4 {
            fight.step(assets, pad(2, true, None), ken, false);
        }
        position_sagat_fixture(&mut fight, 38);
        fight.step(assets, pad(2, true, Some(LP)), ken, false);
        assert_eq!(fight.fight_read().fighters[0].move_id, 0x11);
        for tick in 1..finish {
            let dir = match tick { 2 => 6, 3 => 2, _ => 3 };
            fight.step(assets, pad(dir, true, None), ken, false);
            assert_eq!(fight.fight_read().connected(Side::P1), tick == 5, "opener tick {tick}");
            if tick == 5 {
                assert_eq!(fight.fight_read().blocked[0], guard);
                let freeze = fight.save_netplay_snapshot(0).machine.toggle_state.combo.contact_freeze;
                assert_eq!(freeze, [14, 14, 0]);
            }
        }
        fight.step(assets, pad(6, true, Some(LP)), ken, false);
        assert_eq!(fight.machine().era_super_freeze(), 0);
        assert_eq!(fight.machine().era_fighters()[0].super_meter, 48);
        if finish == 8 {
            assert_eq!(fight.fight_read().fighters[0].move_id, SF2_SPECIAL_SLOT);
            continue;
        }
        assert_eq!(fight.fight_read().fighters[0].move_id, SF2_SUPER_SLOT);
        let mut flash = None;
        let mut contacts = Vec::new();
        for tick in (finish + 1)..=125 {
            fight.step(assets, pad(5, true, None), ken, false);
            if fight.machine().era_super_freeze() != 0 && flash.is_none() {
                assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
                flash = Some(tick);
            }
            if fight.fight_read().connected(Side::P1) {
                contacts.push(tick - 5);
            }
        }
        assert_eq!(flash, Some(20));
        assert_eq!(contacts, [39, 48, 57, 89, 98]);
    }
}

fn close(session: &mut GameSession) {
    for _ in 0..180 {
        let read = session.fight_machine().unwrap().diagnostic_telemetry();
        if read.p2.center_x - read.p1.center_x <= 48 {
            for _ in 0..4 {
                session.step(idle());
            }
            return;
        }
        session.step(p1(6, None));
    }
    panic!("failed to walk into close range");
}

fn command(session: &mut GameSession, directions: &[u8], button: usize) {
    for (index, &direction) in directions.iter().enumerate() {
        let attack = (index + 1 == directions.len()).then_some(button);
        for _ in 0..3 {
            session.step(p1(direction, attack));
        }
    }
}

fn p1_meter(session: &GameSession) -> u8 {
    session.fight_machine().unwrap().era_fighters()[0].super_meter
}

#[test]
fn all_three_sf2_fighters_use_their_projectile_in_each_revision() {
    for fighter in [Ryu, Ken, Sagat] {
        let mut damages = std::collections::BTreeSet::new();
        for (revision, loadout) in [
            (Revision::Ww, Loadout::Standard),
            (Revision::Ce, Loadout::Standard),
            (Revision::Hf, Loadout::Standard),
            (Revision::Super, Loadout::Standard),
            (Revision::St, Loadout::Standard),
            (Revision::St, Loadout::Old),
        ] {
            let Some(select) = support::local_select_with_fighter(
                GaugeMode::EraDefault, fighter, revision, loadout,
            ) else {
                return;
            };
            let mut session = support::local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
            let state = session.fight_machine().unwrap().era_fighters()[0];
            assert_eq!((state.style.revision, state.style.loadout), (revision, loadout));
            assert_eq!(state.gauge().is_some(), (revision, loadout) == (Revision::St, Loadout::Standard));
            let initial_health = session.fight_machine().unwrap().era_fighters()[1].sub_hp;
            command(&mut session, &[2, 3, 6], LP);
            assert_eq!(session.fight().unwrap().fight_read().fighters[0].move_id, 0x1a);
            let damage = (0..120)
                .find_map(|_| {
                    session.step(idle());
                    let health = session.fight_machine().unwrap().era_fighters()[1].sub_hp;
                    (health < initial_health).then(|| initial_health - health)
                })
                .unwrap_or_else(|| panic!("{fighter:?} {revision:?} {loadout:?} projectile did not connect"));
            if matches!(fighter, Ken | Sagat) && revision == Revision::St {
                // The sampled light shots match across both World Turbo 2 loadouts.
                let full_hp = sf_engine::era_fight::FULL_SUB_HP;
                assert_eq!(damage, (16 * full_hp + 72) / 144);
            } else {
                assert!(damages.insert(damage), "{fighter:?} {revision:?} {loadout:?} revision damage");
            }
        }
    }
}

#[test]
fn all_three_super_turbo_supers_work_in_training_show_and_try() {
    for (fighter, name) in [
        (Ryu, "SHINKU HADOKEN"),
        (Ken, "SHORYUREPPA"),
        (Sagat, "TIGER GENOCIDE"),
    ] {
        let Some(mut session) =
            support::training_fight_with_fighter(fighter, Revision::St, Loadout::Standard)
        else {
            return;
        };
        assert_eq!(p1_meter(&session), 48);
        support::assert_training_move_demo(&mut session, name, SF2_SUPER_SLOT, false);
        assert!(
            (0..200).any(|_| {
                session.step(idle());
                p1_meter(&session) == 48
            }),
            "{name} refills after Try recovers"
        );
    }
}

#[test]
fn st_normal_meter_depends_on_strength_and_contact() {
    let Some(base) = fight(Ryu, false) else { return };
    let mut near = base.clone();
    close(&mut near);

    for (button, hit_gain, block_gain) in [(LP, 2, 1), (MP, 4, 3), (HP, 5, 4)] {
        for (start, guard, hit, blocked, gain) in [
            (&base, 5, false, false, 0),
            (&near, 5, true, false, hit_gain),
            (&near, 1, false, true, block_gain),
        ] {
            let mut session = start.clone();
            assert_eq!(p1_meter(&session), 0);
            let mut attacked = false;
            let mut landed = false;
            let mut blockstun = false;
            for frame in 0..48 {
                session.step(FrameInputs {
                    p1: pad(2, true, (frame == 0).then_some(button)),
                    p2: pad(guard, false, None),
                    ..idle()
                });
                let read = session.fight().unwrap().fight_read();
                attacked |= read.fighter(Side::P1).attacking();
                landed |= read.landed_by(Side::P1).is_some_and(|hit| !hit.chip);
                blockstun |= read.fighter(Side::P2).blockstun;
            }
            assert!(attacked, "crouching punch button {button} must start");
            assert_eq!(
                (landed, blockstun),
                (hit, blocked),
                "button {button}, guard {guard}"
            );
            assert_eq!(
                p1_meter(&session),
                gain,
                "button {button}, hit {hit}, blocked {blocked}"
            );
        }
    }
}

#[test]
fn st_special_meter_is_awarded_at_startup_by_move() {
    const QCF: &[u8] = &[2, 3, 6];
    const DP: &[u8] = &[6, 2, 3];
    const QCB: &[u8] = &[2, 1, 4];
    const KNEE: &[u8] = &[2, 3, 6, 9];

    for (fighter, specials) in [
        (Ryu, &[(QCF, LP, 0x1a, 2), (DP, LP, 0x19, 7), (QCB, LK, 0x1b, 6)][..]),
        (Ken, &[(QCF, LP, 0x1a, 2), (DP, LP, 0x19, 8), (QCB, LK, 0x1b, 6)][..]),
        (Sagat, &[
            (QCF, LP, 0x1a, 3),
            (QCF, LK, 0x1b, 3),
            (DP, LP, SF2_SPECIAL_SLOT, 7),
            (KNEE, LK, 0x19, 6),
        ][..]),
    ] {
        let Some(base) = fight(fighter, false) else { continue };
        assert_eq!(p1_meter(&base), 0);
        for &(directions, button, move_id, gain) in specials {
            let mut session = base.clone();
            command(&mut session, directions, button);
            let read = session.fight().unwrap().fight_read();
            assert_eq!(
                read.fighter(Side::P1).move_id,
                move_id,
                "{fighter:?} command {directions:?}, button {button}"
            );
            assert!(read.landed_by(Side::P1).is_none());
            assert_eq!(p1_meter(&session), gain, "{fighter:?} move {move_id:#04x}");
        }
    }
}

#[test]
fn st_hadoken_hit_and_chip_do_not_add_meter() {
    let Some(base) = fight(Ryu, false) else { return };
    for (guard, chip) in [(5, false), (1, true)] {
        let mut session = base.clone();
        command(&mut session, &[2, 3, 6], LP);
        assert_eq!(p1_meter(&session), 2);
        let mut contacts = Vec::new();
        for _ in 0..120 {
            session.step(p2(guard, None));
            if let Some(hit) = session.fight().unwrap().fight_read().landed_by(Side::P1) {
                contacts.push(hit.chip);
            }
        }
        assert_eq!(contacts, [chip]);
        assert_eq!(p1_meter(&session), 2);
    }
}

fn observe_rising_move(mut session: GameSession, id: u8) -> (usize, bool, usize) {
    let mut started = false;
    let mut hits = 0;
    let mut frozen = false;
    let mut launches = 0;
    let mut rising = false;
    for frame in 0..150 {
        let Some(bout) = session.fight() else {
            panic!("move {id:#04x} left Training at frame {frame}: {:?}", session.take_log());
        };
        let read = bout.fight_read();
        let attacker = read.fighter(Side::P1);
        if attacker.move_id == id {
            started = true;
            let just_rising = attacker.state & 3 == 1;
            launches += usize::from(just_rising && !rising);
            rising = just_rising;
            hits += usize::from(read.landed_by(Side::P1).is_some_and(|hit| !hit.chip));
        }
        frozen |= bout.machine().era_super_freeze() != 0;
        session.step(idle());
    }
    assert!(started, "the normal pad input starts {id:#04x}");
    assert!(session.fight_machine().unwrap().unrecovered().is_empty());
    (hits, frozen, launches)
}

#[test]
fn sagat_uses_the_native_tiger_uppercut_and_tiger_genocide_streams() {
    let Some(mut base) = fight(Sagat, true) else { return };
    close(&mut base);
    let assets = base.assets();
    let mut uppercut = base.fight().unwrap().clone();
    let ken = pad(5, false, None);
    for direction in [6, 2] {
        for _ in 0..2 {
            uppercut.step(assets, pad(direction, true, None), ken, false);
        }
    }
    // Training's matchup uses a different defender style, so pin the axes
    // for a close contact instead of the pose-centre approach distance.
    position_sagat_fixture(&mut uppercut, 42);
    uppercut.step(assets, pad(3, true, Some(LP)), ken, false);
    assert_eq!(uppercut.fight_read().fighters[0].move_id, SF2_SPECIAL_SLOT);
    assert_eq!(uppercut.machine().era_super_freeze(), 0);
    let mut uppercut_hit = None;
    for tick in 1..=15 {
        uppercut.step(assets, pad(5, true, None), ken, false);
        if uppercut.fight_read().landed_by(Side::P1).is_some_and(|hit| !hit.chip) {
            uppercut_hit.get_or_insert(tick);
            assert_eq!(uppercut.fight_read().fighters[1].state & 3, 1);
        }
    }
    assert_eq!(uppercut_hit, Some(3));

    command(&mut base, &[2, 3, 6, 2, 3, 6], LP);
    let (super_hits, super_freeze, super_launches) = observe_rising_move(base, SF2_SUPER_SLOT);

    assert!(super_hits >= 2, "Tiger Genocide's knee and uppercut contacts: {super_hits}");
    assert!(super_freeze);
    assert_eq!(super_launches, 2);
}

#[test]
fn tiger_genocide_draws_sagats_own_poses() {
    let Some(mut session) = fight(Sagat, true) else { return };
    close(&mut session);

    let bank = session.data_bank().unwrap();
    let crouch_header = bank.move_header(Sagat, 5).unwrap();
    let crouch = u16::from_be_bytes([bank.rb(crouch_header), bank.rb(crouch_header + 1)]);
    let crouch_slot = sprite_slot(crouch);
    assert!(crouch_slot >= ROM_SPRITE_TILES);
    assert!(crouch_slot < session.fight_sprite_tiles().unwrap());

    let drawn_pose = |session: &GameSession| {
        let machine = session.fight_machine().unwrap();
        let offset = usize::from(machine.diagnostic_telemetry().p1.entity_offset);
        let grid = &machine.state.objectram[offset..];
        u16::from_be_bytes([grid[0], grid[1]])
    };
    for (index, direction) in [2, 3, 6, 2, 3, 6].into_iter().enumerate() {
        for _ in 0..3 {
            session.step(p1(direction, (index == 5).then_some(LP)));
            if direction == 2 {
                assert_eq!(drawn_pose(&session), crouch, "quarter-circle crouch");
            }
        }
    }

    let mut poses = Vec::new();
    for _ in 0..150 {
        if session.fight().unwrap().fight_read().fighter(Side::P1).move_id == SF2_SUPER_SLOT {
            let pose = drawn_pose(&session);
            if poses.last() != Some(&pose) {
                poses.push(pose);
            }
        } else if !poses.is_empty() {
            break;
        }
        session.step(idle());
    }
    assert_eq!(poses.first(), Some(&0x2d80));
    assert!(poses.contains(&0x2d0a), "Sagat's knee pose");
    assert!(poses.contains(&0x2d00), "Sagat's punch pose");
    assert!(poses.contains(&0x2c8c), "Sagat's rising pose");
}

#[test]
fn kens_shoryureppa_advances_into_a_higher_finishing_uppercut() {
    let Some(base) = fight(Ken, true) else { return };
    for at_close_range in [false, true] {
        let mut session = base.clone();
        if at_close_range {
            close(&mut session);
        } else {
            for _ in 0..75 {
                session.step(p1(4, None));
            }
            for _ in 0..4 {
                session.step(idle());
            }
        }
        command(&mut session, &[2, 3, 6, 2, 3, 6], LP);
        let machine = session.fight_machine().unwrap();
        let start = machine.diagnostic_telemetry().p1;
        let start_scroll = machine.state.fg_scroll;
        let start_gap = machine.diagnostic_telemetry().p2.center_x - start.center_x;

        let mut launches = 0;
        let mut was_rising = false;
        let mut frozen = false;
        let mut height = [0; 2];
        let mut airborne_frames = [0; 2];
        let mut hits = [0; 2];
        let mut travel = 0;
        for _ in 0..150 {
            let bout = session.fight().unwrap();
            let read = bout.fight_read();
            let attacker = read.fighter(Side::P1);
            frozen |= bout.machine().era_super_freeze() != 0;
            if attacker.move_id == SF2_SUPER_SLOT {
                let posture = attacker.state & 3;
                let rising = posture == 1;
                launches += usize::from(rising && !was_rising);
                was_rising = rising;
                let machine = bout.machine();
                let position = machine.diagnostic_telemetry().p1;
                let camera_delta = machine.state.fg_scroll.wrapping_sub(start_scroll) as i16;
                travel = position.center_x - start.center_x + i32::from(camera_delta);
                if launches > 0 {
                    let arc = launches - 1;
                    let entity = &machine.state.objectram[usize::from(position.entity_offset)..];
                    let grid_top = i32::from(i16::from_be_bytes([entity[4], entity[5]]));
                    let tile_height = if entity[2] & 4 != 0 { 32 } else { 16 };
                    let grid_bottom = grid_top + (i32::from(entity[0xc]) + 1) * tile_height;
                    height[arc] = height[arc].max(sf_engine::data_bank::GROUND - grid_bottom);
                    airborne_frames[arc] += usize::from(matches!(posture, 1 | 2));
                    hits[arc] += usize::from(read.landed_by(Side::P1).is_some_and(|hit| !hit.chip));
                }
            }
            session.step(idle());
        }

        assert!(frozen);
        assert_eq!(launches, 2);
        // World Turbo 2 uses a 14 px opening hop and a much taller
        // finishing rise. The source whiff peaks 138 px above the floor.
        assert!(height[0] >= 12 && height[1] >= 130, "apex heights: {height:?}");
        assert!(airborne_frames[1] >= 40, "airborne frames: {airborne_frames:?}");
        if at_close_range {
            assert!(hits.iter().all(|&n| n > 0), "{start_gap}px, hits by arc: {hits:?}");
        } else {
            assert_eq!(hits, [0, 0]);
            assert!(travel >= 75, "horizontal travel on whiff: {travel}");
        }
        assert!(session.fight_machine().unwrap().unrecovered().is_empty());
    }
}

#[derive(Debug, Default)]
struct RedTrace {
    saw_three: bool,
    saw_ordinary: bool,
    punched_through: bool,
    hits: usize,
}

fn observe_red(mut session: GameSession, clash: bool) -> RedTrace {
    let mut result = RedTrace::default();
    let mut last_hp = session.fight_machine().unwrap().era_fighters()[1].sub_hp;
    for frame in 0..150 {
        let phase = frame / 3;
        let dir1 = [4, 1, 2, 3, 6].get(phase).copied().unwrap_or(5);
        let dir2 = [5, 5, 2, 3, 6].get(phase).copied().unwrap_or(5);
        let attack = (phase == 4).then_some(LP);
        session.step(FrameInputs {
            p1: pad(dir1, true, attack),
            p2: pad(dir2, false, if clash { attack } else { None }),
            ..idle()
        });
        let machine = session.fight_machine().unwrap();
        let effects = machine.toggle_state().era.projectiles();
        let red = effects.iter().find(|p| p.crimson && p.hits_remaining != 0);
        let ordinary = effects.iter().any(|p| !p.crimson && p.move_id == 0x1a && p.hits_remaining == 1);
        let hp = machine.era_fighters()[1].sub_hp;
        result.hits += usize::from(hp < last_hp);
        last_hp = hp;
        result.saw_three |= red.is_some_and(|p| p.hits_remaining == 3);
        // Both effects can meet on the frame Ken's ordinary projectile spawns.
        // His move ID and the red projectile's spent hit before any HP loss
        // establish that collision even if the ordinary effect ends at once.
        result.saw_ordinary |= session.fight().unwrap().fight_read().fighter(Side::P2).move_id == 0x1a;
        result.punched_through |= result.saw_three && result.saw_ordinary && !ordinary
            && red.is_some_and(|p| p.hits_remaining == 2) && result.hits == 0;
    }
    assert!(session.fight_machine().unwrap().unrecovered().is_empty());
    result
}

#[test]
fn super_revision_keeps_its_existing_red_hadoken_projectile_budget() {
    let Some(select) = support::local_select_with_p1(GaugeMode::EraDefault, Revision::Super, Loadout::Standard) else {
        return;
    };
    let base = support::local_fight(&select, Revision::St, Loadout::Standard);
    let alone = observe_red(base.clone(), false);
    assert!(alone.saw_three, "the native red Hadoken spawns with three hits");
    assert_eq!(alone.hits, 3, "all three contacts land on neutral Ken");
    let clash = observe_red(base, true);
    assert!(clash.punched_through, "the red projectile still has two hits after the clash: {clash:?}");
    assert_eq!(clash.hits, 2, "the surviving projectile spends its remaining hits");
}
