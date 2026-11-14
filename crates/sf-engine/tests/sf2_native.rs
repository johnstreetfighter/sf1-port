//! ADR-0015 through the ordinary menu, native fight, and rollback entry points.

use std::path::Path;

use sf_engine::config::MatchConfig;
use sf_engine::era::{Loadout, Revision};
use sf_engine::era_fight::{
    FULL_SUB_HP, SF2_SUPER, ST_KEN_SHORYUREPPA_FREEZE, ST_RYU_SHINKU_FREEZE, SUPER_FREEZE_FRAMES,
};
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
        if revision == Revision::Sf3ThreeS {
            assert_eq!(loadout, Loadout::Art1);
            for _ in 0..3 {
                side_tap(session, side, 1, false);
            }
        } else {
            side_tap(session, side, 1, false); // SF2 ST
        }
        if revision == Revision::Sfz3 {
            assert_eq!(loadout, Loadout::AIsm);
            side_tap(session, side, 1, false); // SFZ3 A-ISM
        } else if !matches!(revision, Revision::St | Revision::Sf3ThreeS) {
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
    let startup = fight.step(&assets, pad(1, true), pad(0, false), false);
    assert!(!startup.announcements.contains(&AnnouncerLine::Super));
    assert_eq!(fight.fight_read().fighters[0].move_id, SF2_SUPER);
    assert_eq!(fight.machine().era_fighters()[0].super_meter, GAUGE_FULL);
    for _ in 0..8 {
        fight.step(&assets, pad(0, false), pad(0, false), false);
        assert_eq!(fight.machine().era_super_freeze(), 0);
    }
    let activation = fight.step(&assets, pad(0, false), pad(0, false), false);
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
    assert_eq!(fight.machine().era_super_freeze(), ST_RYU_SHINKU_FREEZE - 1);
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
    for left in (0..ST_RYU_SHINKU_FREEZE - 1).rev() {
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
        let freeze = if revision == Revision::St { ST_KEN_SHORYUREPPA_FREEZE } else { SUPER_FREEZE_FRAMES };
        assert_eq!(fight.machine().era_super_freeze(), freeze - 1);
        let opening = grid(&fight);
        for _ in 0..freeze {
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
fn st_ken_shoryureppa_counts_from_second_down_forward_and_old_uses_dp() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let st = Some((Revision::St, Loadout::Standard));
    let (assets, mut new) = native_fight([None, st], false);
    let (old_assets, old) = native_fight([None, Some((Revision::St, Loadout::Old))], false);
    let idle = PlayerInputs::default();
    let mut stock = new.save_netplay_snapshot(0);
    stock.machine.toggle_state.era.fighters[1].super_meter = GAUGE_FULL;
    new.restore_netplay_snapshot(&stock);
    for (base, assets, old) in [(&new, &assets, false), (&old, &old_assets, true)] {
        let mut initial = base.clone();
        for dir in [4, 6, 2, 4] {
            initial.step(assets, idle, pad(dir, false), false);
        }
        for button in 0..3 {
            let cases: &[(u8, u8)] = if old { &[(0, 0x19), (1, 0x19)] }
                else { &[(0, SF2_SUPER), (1, SF2_SUPER), (10, SF2_SUPER), (11, 0x19)] };
            for &(delay, expected) in cases {
                let mut fight = initial.clone();
                for tick in 0..=delay {
                    let mut input = pad(if tick == 0 { 6 } else { 2 }, false);
                    input.buttons[button] = tick == delay;
                    fight.step(assets, idle, input, false);
                }
                assert_eq!(fight.fight_read().fighters[1].move_id, expected, "Old {old}, button {button}, +{delay}");
                if expected == SF2_SUPER {
                    for freeze in (0..ST_KEN_SHORYUREPPA_FREEZE).rev() {
                        assert_eq!(fight.machine().era_super_freeze(), freeze);
                        fight.step(assets, idle, idle, false);
                    }
                } else {
                    assert_eq!(fight.machine().era_super_freeze(), 0);
                }
            }
        }
    }
}

#[test]
fn st_ken_crouching_mk_buffers_shoryureppa_through_contact_freeze() {
    use sf_video::callouts::Side;
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let st = Some((Revision::St, Loadout::Standard));
    let (assets, mut initial) = native_fight([st, st], false);
    let mut stock = initial.save_netplay_snapshot(0);
    stock.machine.toggle_state.era.fighters[1].super_meter = GAUGE_FULL;
    initial.restore_netplay_snapshot(&stock);
    let idle = PlayerInputs::default();
    for dir in [4, 6, 2, 4] {
        initial.step(&assets, idle, pad(dir, false), false);
    }
    // The arcade cancel recipe holds the second Down until the first
    // quarter circle no longer accepts the command kick's MK edge.
    for _ in 1..10 {
        initial.step(&assets, idle, pad(4, false), false);
    }
    set_standing_gap(&mut initial, 32);
    let mut kick = pad(4, false);
    kick.buttons[4] = true;
    initial.step(&assets, idle, kick, false);
    assert_eq!(initial.fight_read().fighters[1].move_id, 0x17);
    for tick in 1..=4 {
        initial.step(&assets, idle, pad(4, false), false);
        assert_eq!(initial.fight_read().connected(Side::P2), tick == 4);
    }
    for (delay, accepted) in [(1, true), (3, true), (4, false)] {
        let mut fight = initial.clone();
        for _ in 1..delay {
            fight.step(&assets, idle, pad(4, false), false);
        }
        fight.step(&assets, idle, pad(6, true), false);
        assert_eq!(fight.fight_read().fighters[1].move_id == SF2_SUPER, accepted, "contact +{delay}");
        assert_eq!(fight.machine().era_fighters()[1].super_meter, GAUGE_FULL);
        for tick in delay + 1..=15 {
            fight.step(&assets, idle, idle, false);
            let flash = fight.machine().era_super_freeze();
            assert_eq!(flash, if accepted && tick == 15 { ST_KEN_SHORYUREPPA_FREEZE - 1 } else { 0 },
                "contact +{delay}, flash tick {tick}");
        }
        assert_eq!(fight.machine().era_fighters()[1].super_meter, if accepted { 0 } else { GAUGE_FULL });
    }
}

#[test]
fn st_ken_shoryureppa_uses_world_strike_phases_on_hit_guard_and_whiff() {
    use sf_video::callouts::Side;
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let st = Some((Revision::St, Loadout::Standard));
    let (assets, mut initial) = native_fight([st, st], false);
    let mut stock = initial.save_netplay_snapshot(0);
    stock.machine.toggle_state.era.fighters[1].super_meter = GAUGE_FULL;
    initial.restore_netplay_snapshot(&stock);
    let idle = PlayerInputs::default();
    for dir in [4, 6, 2, 4] {
        initial.step(&assets, idle, pad(dir, false), false);
    }
    let close_hits = &[(19, 100), (29, 107), (56, 107), (66, 113), (76, 107)][..];
    for (gap, guard, expected_contacts, expected_recovery) in [
        (32, false, close_hits, 143),
        (55, false, close_hits, 143),
        (90, false, &[(22, 107), (48, 107), (58, 113), (68, 107)], 135),
        (170, false, &[(48, 107)], 111),
        (232, false, &[], 103),
        (32, true, &[(19, 27), (29, 27), (58, 27), (68, 27)], 135),
        (55, true, &[(19, 27), (29, 27), (58, 27), (72, 27)], 135),
    ] {
        let mut fight = initial.clone();
        set_standing_gap(&mut fight, gap);
        if guard {
            // Give Ryu room for the full guard recoil and include the arcade
            // fixture's eight pixels of retreat before standing guard.
            let mut position = fight.save_netplay_snapshot(0);
            let center_shift = 284_u16.wrapping_sub(center_x(&fight, P1_STRUCT));
            shift_test_fighter(&mut position, P1_STRUCT, center_shift);
            shift_test_fighter(&mut position, P2_STRUCT, center_shift + 8);
            fight.restore_netplay_snapshot(&position);
        }
        let ryu = if guard { pad(2, false) } else { idle };
        fight.step(&assets, ryu, pad(6, true), false);
        let mut contacts = Vec::new();
        let mut recovery = None;
        let mut previous_hp = fight.machine().era_fighters()[0].sub_hp;
        let mut guard_ready = None;
        let mut guard_snapshot = None;
        for tick in 1..=145 {
            fight.step(&assets, ryu, idle, false);
            let read = fight.fight_read();
            let ken = read.fighters[1];
            if gap == 232 {
                let active = [19..21, 21..27, 40..42, 42..44, 44..70]
                    .into_iter().any(|phase| phase.contains(&tick));
                assert_eq!(ken.attack_active, active, "whiff tick {tick}");
            }
            let hp = fight.machine().era_fighters()[0].sub_hp;
            if read.connected(Side::P2) {
                assert_eq!(read.blocked[1], guard, "gap {gap}, tick {tick}");
                contacts.push((tick, previous_hp - hp));
            } else {
                assert_eq!(hp, previous_hp, "gap {gap}, tick {tick}");
            }
            previous_hp = hp;
            if guard {
                if gap == 55 && tick == 29 {
                    guard_snapshot = Some(fight.save_netplay_snapshot(tick));
                }
                if contacts.len() == expected_contacts.len() && !read.fighters[0].blockstun {
                    guard_ready.get_or_insert(tick);
                }
            }
            if !ken.attacking() {
                recovery.get_or_insert(tick);
            }
        }
        assert_eq!(contacts, expected_contacts, "gap {gap}, guard {guard}");
        assert_eq!(recovery, Some(expected_recovery), "gap {gap}, guard {guard}");
        if guard {
            let (last_contact, _) = expected_contacts.last().unwrap();
            assert_eq!(guard_ready, Some(last_contact + 31), "guard gap {gap}");
        }
        if let Some(snapshot) = guard_snapshot {
            let expected = fight.state_checksum();
            fight.restore_netplay_snapshot(&snapshot);
            for _ in 30..=145 {
                fight.step(&assets, ryu, idle, false);
            }
            assert_eq!(fight.state_checksum(), expected, "guard rollback");
        }
    }
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

fn st_standing_gap(fight: &Fight) -> u16 {
    center_x(fight, P1_STRUCT).abs_diff(center_x(fight, P2_STRUCT))
}

fn set_standing_gap(fight: &mut Fight, gap: u16) {
    let shift = gap.wrapping_sub(st_standing_gap(fight));
    let mut snapshot = fight.save_netplay_snapshot(0);
    shift_test_fighter(&mut snapshot, P2_STRUCT, shift);
    fight.restore_netplay_snapshot(&snapshot);
    assert_eq!(st_standing_gap(fight), gap);
}

fn shift_test_fighter(snapshot: &mut sf_engine::front::fight::FightSnapshot, ps: u32, shift: u16) {
    let offset = (ps - 0xff8000) as usize + 0x10;
    let e = u16::from_be_bytes(snapshot.machine.work[offset..offset + 2].try_into().unwrap()) as usize;
    let objects = &mut snapshot.machine.state.objectram;
    let cells = (usize::from(objects[e + 0xc]) + 1) * (usize::from(objects[e + 0xd]) + 1);
    for sprite in (e..).step_by(0x40).take(cells) {
        let x = &mut objects[sprite + 6..sprite + 8];
        let moved = u16::from_be_bytes(x.try_into().unwrap()).wrapping_add(shift);
        x.copy_from_slice(&moved.to_be_bytes());
    }
}

#[test]
fn st_shinku_replays_turbo_two_motion_flash_and_ground_contacts() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let st = Some((Revision::St, Loadout::Standard));
    let (assets, initial) = native_fight([st, st], false);
    let idle = PlayerInputs::default();
    for (gap, expected) in [(32, &[28, 38, 48][..]), (90, &[28, 38, 48, 58, 68])] {
        let mut fight = initial.clone();
        for direction in [4, 5, 1, 4, 5, 1] {
            fight.step(&assets, pad(direction, false), idle, false);
        }
        set_standing_gap(&mut fight, gap);
        let mut snapshot = fight.save_netplay_snapshot(0);
        snapshot.machine.toggle_state.era.fighters[0].super_meter = GAUGE_FULL;
        fight.restore_netplay_snapshot(&snapshot);
        for _ in 1..10 {
            fight.step(&assets, idle, idle, false);
        }
        let mut late = fight.clone();
        late.step(&assets, idle, idle, false);
        late.step(&assets, pad(0, true), idle, false);
        assert_ne!(late.fight_read().fighters[0].move_id, SF2_SUPER);

        fight.step(&assets, pad(0, true), idle, false);
        assert_eq!(fight.fight_read().fighters[0].move_id, SF2_SUPER);
        let opening = fight.save_netplay_snapshot(0);
        let mut replay = fight.clone();
        for _ in 0..39 {
            replay.step(&assets, idle, idle, false);
        }
        let checksum = replay.state_checksum();
        replay.restore_netplay_snapshot(&opening);
        for _ in 0..39 {
            replay.step(&assets, idle, idle, false);
        }
        assert_eq!(replay.state_checksum(), checksum);

        let mut hp = FULL_SUB_HP;
        let mut contacts = Vec::new();
        let mut launch = None;
        let mut neutral = None;
        let mut recovered = None;
        for tick in 1..=195 {
            fight.step(&assets, idle, idle, false);
            let freeze = fight.machine().era_super_freeze();
            assert_eq!(freeze, if (9..27).contains(&tick) { 26 - tick } else { 0 });
            if fight.fight_read().fighters[0].move_id == 1 {
                recovered.get_or_insert(tick);
            }
            let current_hp = fight.machine().era_fighters()[1].sub_hp;
            if current_hp < hp {
                contacts.push(tick);
                hp = current_hp;
            }
            let ken = fight.fight_read().fighters[1];
            if ken.move_id == 0x35 {
                launch.get_or_insert(tick);
            }
            if !contacts.is_empty() && ken.move_id == 1 {
                neutral.get_or_insert(tick);
            }
        }
        assert_eq!(contacts, expected, "{gap} px");
        assert_eq!(recovered, Some(69));
        assert_eq!(launch, if gap == 90 { Some(68) } else { None });
        assert_eq!(neutral, Some(if gap == 90 { 188 } else { 74 }));
    }
}

#[test]
fn st_shinku_accepts_all_punches_and_late_held_forward_falls_back_to_hadoken() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let st = Some((Revision::St, Loadout::Standard));
    let (assets, mut initial) = native_fight([st, st], false);
    let idle = PlayerInputs::default();
    let mut stock = initial.save_netplay_snapshot(0);
    stock.machine.toggle_state.era.fighters[0].super_meter = GAUGE_FULL;
    initial.restore_netplay_snapshot(&stock);
    for dir in [4, 5, 1, 4, 5] {
        initial.step(&assets, pad(dir, false), idle, false);
    }
    for button in 0..3 {
        let mut punch = pad(1, false);
        punch.buttons[button] = true;
        for (offset, expected) in [(0, SF2_SUPER), (10, SF2_SUPER), (11, 0x1a)] {
            let mut fight = initial.clone();
            for _ in 0..offset {
                fight.step(&assets, pad(1, false), idle, false);
            }
            fight.step(&assets, punch, idle, false);
            assert_eq!(fight.fight_read().fighters[0].move_id, expected,
                "punch {button}, completion offset {offset}");
            assert_eq!(fight.machine().era_fighters()[0].super_meter, GAUGE_FULL);
        }
    }
}

#[test]
fn st_shinku_crouching_kick_gate_counts_through_contact_freeze() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let st = Some((Revision::St, Loadout::Standard));
    let (assets, mut initial) = native_fight([st, st], false);
    set_standing_gap(&mut initial, 32);
    let idle = PlayerInputs::default();
    let mut stock = initial.save_netplay_snapshot(0);
    stock.machine.toggle_state.era.fighters[0].super_meter = GAUGE_FULL;
    initial.restore_netplay_snapshot(&stock);
    for (button, guard) in [(3, idle), (4, pad(5, false))] {
        for (delay, accepted) in [(5, true), (6, true), (7, true), (8, false)] {
            let mut fight = initial.clone();
            for dir in [4, 5, 1, 4] {
                fight.step(&assets, pad(dir, false), guard, false);
            }
            set_standing_gap(&mut fight, 32);
            let mut kick = pad(5, false);
            kick.buttons[button] = true;
            fight.step(&assets, kick, guard, false);
            assert_eq!(fight.fight_read().fighters[0].move_id, 0x17);
            for tick in 1..delay {
                fight.step(&assets, pad(5, false), guard, false);
                let hitstop = fight.machine().toggle_state().combo.contact_freeze[0];
                assert_eq!(hitstop != 0, tick >= 4,
                    "kick {button}, action offset {tick}, read {:?}", fight.fight_read().fighters);
            }
            fight.step(&assets, pad(1, true), guard, false);
            assert_eq!(fight.fight_read().fighters[0].move_id == SF2_SUPER, accepted,
                "kick {button}, action offset {delay}");
            assert_eq!(fight.machine().era_fighters()[0].super_meter, GAUGE_FULL);
            if accepted {
                for tick in delay + 1..=28 {
                    fight.step(&assets, idle, idle, false);
                    let freeze = fight.machine().era_super_freeze();
                    assert_eq!(freeze, if tick == 28 { ST_RYU_SHINKU_FREEZE - 1 } else { 0 },
                        "kick {button}, delayed start at action offset {tick}");
                }
            }
        }
    }
}

#[test]
fn st_shinku_uses_one_cancel_clock_for_all_eighteen_grounded_normals() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let st = Some((Revision::St, Loadout::Standard));
    let (assets, mut initial) = native_fight([st, st], false);
    let mut stock = initial.save_netplay_snapshot(0);
    stock.machine.toggle_state.era.fighters[0].super_meter = GAUGE_FULL;
    initial.restore_netplay_snapshot(&stock);
    let idle = PlayerInputs::default();
    let far = [65, 71, 81, 41, 41, 96];
    for (button, far_gap) in far.into_iter().enumerate() {
        for (crouch, gap) in [(false, 32), (false, far_gap), (true, 32)] {
            for whiff in [false, true] {
                let mut opening = initial.clone();
                set_standing_gap(&mut opening, gap);
                let mut attack = pad(if crouch { 4 } else { 0 }, false);
                attack.buttons[button] = true;
                opening.step(&assets, attack, idle, false);
                let opener = opening.fight_read().fighters[0];
                assert!(opener.attacking() && opener.move_id != 0x1a,
                    "ordinary opener for button {button}, crouch {crouch}, gap {gap}: {opener:?}");
                if whiff {
                    set_standing_gap(&mut opening, 170);
                }
                for (delay, accepted) in [(7, true), (8, false)] {
                    let mut fight = opening.clone();
                    let motion = if crouch { [5, 1, 4, 5, 1, 0] } else { [4, 5, 1, 4, 5, 1] };
                    for tick in 1..delay {
                        let dir = motion.get(tick - 1).copied().unwrap_or(0);
                        fight.step(&assets, pad(dir, false), idle, false);
                    }
                    let connected = fight.machine().era_fighters()[1].sub_hp < FULL_SUB_HP;
                    if whiff {
                        assert!(!connected);
                    }
                    fight.step(&assets, pad(1, true), idle, false);
                    assert_eq!(
                        fight.fight_read().fighters[0].move_id == SF2_SUPER,
                        accepted,
                        "button {button}, crouch {crouch}, gap {gap}, whiff {whiff}, offset {delay}",
                    );
                    assert_eq!(fight.machine().toggle_state().cancel.count[0], u8::from(accepted));
                }
            }
        }
    }
}

#[test]
fn st_shinku_follows_a_clean_crouching_mk_as_a_true_combo() {
    use sf_video::callouts::Side;
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let st = Some((Revision::St, Loadout::Standard));
    let (assets, mut fight) = native_fight([st, st], false);
    let idle = PlayerInputs::default();
    let mut stock = fight.save_netplay_snapshot(0);
    stock.machine.toggle_state.era.fighters[0].super_meter = GAUGE_FULL;
    fight.restore_netplay_snapshot(&stock);
    for dir in [4, 5, 1, 4] {
        fight.step(&assets, pad(dir, false), idle, false);
    }
    set_standing_gap(&mut fight, 32);
    let mut kick = pad(5, false);
    kick.buttons[4] = true;
    fight.step(&assets, kick, idle, false);
    let mut opener = None;
    let mut super_hit = None;
    for tick in 1..=90 {
        let input = match tick {
            1..=5 => pad(5, false),
            6 => pad(1, true),
            _ => idle,
        };
        fight.step(&assets, input, idle, false);
        let read = fight.fight_read();
        if let Some(hit) = read.landed_by(Side::P1) {
            if read.fighters[0].move_id == SF2_SUPER {
                super_hit = Some((tick, hit.into_hitstun));
                break;
            }
            assert_eq!(read.fighters[1].state & 3, 2, "MK keeps Ken grounded");
            assert!(opener.replace(tick).is_none());
        }
    }
    assert_eq!(opener, Some(4));
    assert_eq!(super_hit, Some((47, true)));
}

#[test]
fn st_shinku_has_one_close_crouch_guard_contact_and_five_at_ninety_pixels() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let st = Some((Revision::St, Loadout::Standard));
    let (assets, mut initial) = native_fight([st, st], false);
    let idle = PlayerInputs::default();
    let guard = pad(5, false);
    let mut stock = initial.save_netplay_snapshot(0);
    stock.machine.toggle_state.era.fighters[0].super_meter = GAUGE_FULL;
    initial.restore_netplay_snapshot(&stock);
    for dir in [4, 5, 1, 4, 5] {
        initial.step(&assets, pad(dir, false), guard, false);
    }
    for (gap, expected) in [(32u16, &[28][..]), (90, &[28, 38, 48, 58, 68])] {
        let mut fight = initial.clone();
        fight.step(&assets, pad(1, true), guard, false);
        let mut hp = FULL_SUB_HP;
        let mut chips = Vec::new();
        for tick in 1..=105 {
            if tick == 28 {
                let machine = fight.machine();
                let read = machine.diagnostic_telemetry();
                let objects = &machine.state.objectram;
                let pose_x = |entity: u16| i16::from(objects[usize::from(entity) + 0x26] as i8);
                let offset = pose_x(read.p2.entity_offset) - pose_x(read.p1.entity_offset);
                set_standing_gap(&mut fight, gap.wrapping_add_signed(offset));
            }
            fight.step(&assets, idle, guard, false);
            let current_hp = fight.machine().era_fighters()[1].sub_hp;
            if current_hp < hp {
                chips.push(tick);
                assert_eq!(hp - current_hp, 20, "{gap} px, contact at {tick}");
                hp = current_hp;
            }
            assert_ne!(fight.fight_read().fighters[1].move_id, 0x35);
        }
        assert_eq!(chips, expected, "{gap} px");
    }
}

#[test]
fn st_shinku_airborne_hit_freezes_and_replays_across_projectile_rehit() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let st = Some((Revision::St, Loadout::Standard));
    let (assets, mut initial) = native_fight([st, st], false);
    let mut stock = initial.save_netplay_snapshot(0);
    stock.machine.toggle_state.era.fighters[0].super_meter = GAUGE_FULL;
    initial.restore_netplay_snapshot(&stock);
    let idle = PlayerInputs::default();
    for dir in [4, 5, 1, 4, 5] {
        initial.step(&assets, pad(dir, false), idle, false);
    }
    set_standing_gap(&mut initial, 60);
    initial.step(&assets, pad(1, true), idle, false);
    let mut fight = initial;
    // Start Ken's jump early enough to finish his World ST prejump before contact.
    for tick in 1..28 {
        fight.step(&assets, idle, pad(if (2..5).contains(&tick) { 8 } else { 0 }, false), false);
    }
    assert_eq!(fight.fight_read().fighters[1].state & 3, 1);
    let mut snapshot = None;
    for tick in 28..=38 {
        let hp = fight.machine().era_fighters()[1].sub_hp;
        fight.step(&assets, idle, idle, false);
        assert_eq!(fight.fight_read().fighters[1].state & 3, 1);
        if matches!(tick, 28 | 38) {
            assert!(fight.machine().era_fighters()[1].sub_hp < hp);
            assert_eq!(fight.machine().toggle_state().combo.contact_freeze[..2], [0, 14]);
        } else {
            assert_eq!(fight.machine().era_fighters()[1].sub_hp, hp);
        }
        if tick == 28 {
            snapshot = Some(fight.save_netplay_snapshot(tick));
        }
    }
    let checksum = fight.state_checksum();
    fight.restore_netplay_snapshot(&snapshot.unwrap());
    for _ in 0..10 {
        fight.step(&assets, idle, idle, false);
    }
    assert_eq!(fight.state_checksum(), checksum);

    // Wait for Shinku's own recovery before issuing a fresh jump attack.
    for (jump, press, expected) in [(70, 7, Some(83)), (90, 19, None)] {
        let mut replay = fight.clone();
        let mut hits = Vec::new();
        for tick in 39..=125 {
            let mut input = pad(if (jump..jump + 3).contains(&tick) { 9 } else { 0 }, false);
            input.buttons[1] = tick == jump + press;
            let airborne = replay.fight_read().fighters[1].state & 3 == 1;
            replay.step(&assets, input, idle, false);
            if let Some(hit) = replay.fight_read().landed_by(sf_video::callouts::Side::P1) {
                assert!(airborne && hit.into_hitstun);
                hits.push(tick);
            }
        }
        assert_eq!(hits, expected.into_iter().collect::<Vec<_>>(), "jump +{jump}, MP +{press}");
    }
}

#[test]
fn st_ken_crouching_light_and_medium_kicks_match_the_world_ground_reactions() {
    use sf_video::callouts::Side;
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let idle = PlayerInputs::default();
    for loadout in [Loadout::Standard, Loadout::Old] {
        let (assets, base) = native_fight([
            Some((Revision::St, Loadout::Standard)), Some((Revision::St, loadout)),
        ], false);
        for (button, push_steps) in [
            (3, &[4, 4, 4, 4, 3, 3, 3, 2, 1, 1, 1][..]),
            (4, &[6, 6, 6, 5, 5, 5, 4, 4, 3, 3, 2, 1, 1, 1, 1, 1][..]),
        ] {
            let startup = if button == 3 && loadout == Loadout::Old { 3 } else { 4 };
            for guard in [false, true] {
                let mut fight = base.clone();
                let ryu_input = if guard { pad(6, false) } else { idle };
                if guard {
                    fight.step(&assets, ryu_input, idle, false);
                }
                set_standing_gap(&mut fight, 32);
                let mut kick = pad(4, false);
                kick.buttons[button] = true;
                fight.step(&assets, ryu_input, kick, false);
                for tick in 1..=startup {
                    fight.step(&assets, ryu_input, idle, false);
                    let read = fight.fight_read();
                    assert_eq!(read.connected(Side::P2), tick == startup,
                        "{loadout:?}, kick {button}, guard {guard}, tick {tick}");
                }
                let contact = fight.fight_read();
                assert_eq!(contact.blocked[1], guard);
                assert_eq!(contact.fighters[0].health == 48, guard);
                assert_eq!(fight.machine().toggle_state().combo.contact_freeze[..2], [14, 14]);
                let world_x = |fight: &Fight| {
                    let machine = fight.machine();
                    let ryu = machine.diagnostic_telemetry().p1;
                    let camera = u16::from_be_bytes(machine.work_ram()[0x400c..0x400e].try_into().unwrap());
                    let offset = machine.state.objectram[usize::from(ryu.entity_offset) + 0x26] as i8;
                    i32::from(camera) + ryu.center_x - i32::from(offset)
                };
                let origin = world_x(&fight);
                let mut distance = 0;
                for tick in 1..=16 + push_steps.len() {
                    fight.step(&assets, idle, idle, false);
                    if let Some(step) = tick.checked_sub(16).and_then(|index| push_steps.get(index)) {
                        distance += step;
                    }
                    let ryu = fight.fight_read().fighters[0];
                    assert_ne!(ryu.state & 3, 1);
                    assert_eq!(world_x(&fight), origin - distance,
                        "{loadout:?}, kick {button}, guard {guard}, recoil tick {tick}");
                    if tick == 16 + push_steps.len() {
                        assert!(!ryu.hitstun() && !ryu.blockstun);
                    }
                }
                let checksum = fight.state_checksum();
                let snapshot = fight.save_netplay_snapshot(0);
                fight.step(&assets, idle, idle, false);
                let advanced = fight.state_checksum();
                fight.restore_netplay_snapshot(&snapshot);
                assert_eq!(fight.state_checksum(), checksum);
                fight.step(&assets, idle, idle, false);
                assert_eq!(fight.state_checksum(), advanced);
            }
        }
    }
}

#[test]
fn st_ken_crouching_kicks_use_the_world_whiff_intervals() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let idle = PlayerInputs::default();
    for (loadout, lk_start) in [(Loadout::Standard, 4), (Loadout::Old, 3)] {
        let (assets, base) = native_fight([
            Some((Revision::St, Loadout::Standard)), Some((Revision::St, loadout)),
        ], false);
        for (button, live, attack_end) in [(3, lk_start..lk_start + 4, lk_start + 9), (4, 4..10, 19)] {
            let mut fight = base.clone();
            set_standing_gap(&mut fight, 170);
            let mut kick = pad(4, false);
            kick.buttons[button] = true;
            fight.step(&assets, idle, kick, false);
            for tick in 1..=attack_end {
                fight.step(&assets, idle, idle, false);
                let ken = fight.fight_read().fighters[1];
                assert_eq!(ken.attack_active, live.contains(&tick), "{loadout:?} button {button} at {tick}");
                assert_eq!(ken.attacking(), tick < attack_end, "{loadout:?} button {button} at {tick}");
            }
        }
    }
}

#[test]
fn st_ken_crouching_medium_kick_respects_a_third_strike_low_parry() {
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let (assets, mut fight) = native_fight([
        Some((Revision::Sf3ThreeS, Loadout::Art1)), Some((Revision::St, Loadout::Standard)),
    ], false);
    assert_eq!(fight.machine().era_fighters()[0].style.revision, Revision::Sf3ThreeS);
    let idle = PlayerInputs::default();
    set_standing_gap(&mut fight, 32);
    let mut kick = pad(4, false);
    kick.buttons[4] = true;
    fight.step(&assets, idle, kick, false);
    for tick in 1..=7 {
        fight.step(&assets, if tick == 3 { pad(4, false) } else { idle }, idle, false);
        if fight.fight_read().fighters[0].move_id == 0x2e {
            assert_eq!(fight.machine().era_fighters()[0].sub_hp, FULL_SUB_HP);
            assert_eq!(fight.machine().toggle_state().combo.contact_freeze[..2], [0, 0]);
            return;
        }
    }
    panic!("New Ken's crouching MK did not reach Ryu's low parry");
}

#[test]
fn st_ken_crouching_kicks_cancel_to_light_hadoken_at_the_world_boundary() {
    use sf_video::callouts::Side;
    if !Path::new(ROMS).is_dir() {
        return;
    }
    let idle = PlayerInputs::default();
    for loadout in [Loadout::Standard, Loadout::Old] {
        let (assets, base) = native_fight([
            Some((Revision::St, Loadout::Standard)), Some((Revision::St, loadout)),
        ], false);
        for (button, last) in [(3, if loadout == Loadout::Old { 17 } else { 16 }), (4, 16)] {
            let mut opening = base.clone();
            set_standing_gap(&mut opening, 32);
            let mut kick = pad(4, false);
            kick.buttons[button] = true;
            opening.step(&assets, idle, kick, false);
            for _ in 0..8 {
                if opening.fight_read().connected(Side::P2) {
                    break;
                }
                opening.step(&assets, idle, pad(4, false), false);
            }
            assert!(opening.fight_read().connected(Side::P2));
            for delay in [2, last, last + 1] {
                let mut fight = opening.clone();
                let mut first_hit = None;
                let mut fireball_start = None;
                for tick in 1..=55 {
                    let input = if delay == 2 {
                        // The first Down is already held at contact. The
                        // earliest port replay gives the remaining diagonal
                        // and forward on separate controller ticks.
                        match tick {
                            1 => pad(6, false),
                            2 => pad(2, true),
                            _ => idle,
                        }
                    } else if tick + 2 == delay {
                        pad(4, false)
                    } else if tick + 1 == delay {
                        pad(6, false)
                    } else if tick == delay {
                        pad(2, true)
                    } else {
                        idle
                    };
                    fight.step(&assets, idle, input, false);
                    let read = fight.fight_read();
                    if read.fighters[1].move_id == 0x1a {
                        fireball_start.get_or_insert(tick);
                    }
                    if let Some(hit) = read.landed_by(Side::P2) {
                        first_hit.get_or_insert((tick, hit.into_hitstun));
                    }
                }
                let tag = format!("{loadout:?} button {button} at contact +{delay}, start {fireball_start:?}");
                let accepted = delay <= last;
                assert_eq!(fight.machine().toggle_state().cancel.count[1], u8::from(accepted), "{tag}");
                let expected = if !accepted {
                    None
                } else if button == 3 && delay == last {
                    Some((29, false))
                } else {
                    let cast = delay.max(15);
                    Some((cast + if loadout == Loadout::Old { 12 } else { 13 }, button == 4))
                };
                assert_eq!(first_hit, expected, "{tag}");
                if !accepted && button == 4 {
                    for dir in [4, 6, 2] {
                        fight.step(&assets, idle, pad(dir, dir == 2), false);
                    }
                    assert_eq!(fight.fight_read().fighters[1].move_id, 0x1a);
                    let elapsed = (1..=16).find(|_| {
                        fight.step(&assets, idle, idle, false);
                        fight.fight_read().landed_by(Side::P2).is_some_and(|hit| !hit.into_hitstun)
                    });
                    assert_eq!(elapsed, Some(if loadout == Loadout::Old { 12 } else { 13 }));
                }
            }
        }
    }
}

#[test]
fn st_ken_hadoken_strengths_follow_world_turbo_two() {
    use sf_video::callouts::Side;

    if !Path::new(ROMS).is_dir() {
        return;
    }
    let idle = PlayerInputs::default();
    for loadout in [Loadout::Standard, Loadout::Old] {
        let (assets, base) = native_fight([
            Some((Revision::St, Loadout::Standard)), Some((Revision::St, loadout)),
        ], false);
        for strength in 1..=3 {
            for (gap, guard) in [(171, false), (104, false), (104, true)] {
                let mut fight = base.clone();
                set_standing_gap(&mut fight, gap);
                let defender = if guard { pad(6, false) } else { idle };
                for _ in 0..4 {
                    fight.step(&assets, defender, idle, false);
                }
                for dir in [4, 6] {
                    fight.step(&assets, defender, pad(dir, false), false);
                }
                let mut cast = pad(2, false);
                cast.buttons[strength - 1] = true;
                fight.step(&assets, defender, cast, false);
                let case = format!("{loadout:?}, strength {strength}, gap {gap}, guard {guard}");
                assert_eq!(fight.fight_read().fighters[1].move_id, 0x1a, "{case}");
                let release = if loadout == Loadout::Old { 12 } else { 13 };
                let ready = release + 40;
                let mut previous_x = None;
                let mut hp = FULL_SUB_HP;
                let mut hits = 0;
                let whiff = gap == 171;
                for tick in 1..=ready {
                    if whiff && tick <= release + 36 {
                        // Keep the opponent out of the collision test, like
                        // the arcade capture with Ryu pinned above the shot.
                        let mut snapshot = fight.save_netplay_snapshot(tick);
                        let owner = (P1_STRUCT - 0xff8000) as usize + 0x10;
                        let slot = u16::from_be_bytes(snapshot.machine.work[owner..owner + 2].try_into().unwrap()) as usize;
                        let objects = &mut snapshot.machine.state.objectram;
                        objects[slot + 0xf] |= 8;
                        fight.restore_netplay_snapshot(&snapshot);
                    }
                    let checkpoint = if whiff && tick == release + 2 {
                        Some(fight.save_netplay_snapshot(tick))
                    } else {
                        None
                    };
                    fight.step(&assets, defender, idle, false);
                    if let Some(checkpoint) = checkpoint {
                        let digest = fight.state_checksum();
                        fight.restore_netplay_snapshot(&checkpoint);
                        fight.step(&assets, defender, idle, false);
                        assert_eq!(fight.state_checksum(), digest, "{case}, tick {tick}");
                    }
                    let machine = fight.machine();
                    let read = fight.fight_read();
                    if whiff && tick < release + 4 {
                        let shot = machine.toggle_state().era.projectiles().iter()
                            .position(|shot| shot.move_id == 0x1a && shot.hits_remaining > 0);
                        assert_eq!(shot.is_some(), tick >= release, "{case}, tick {tick}");
                        if let Some(index) = shot {
                            let offset = 0x100 + index * 0x40 + 6;
                            let bytes = &machine.state.objectram[offset..offset + 2];
                            let x = u16::from_be_bytes(bytes.try_into().unwrap());
                            if let Some(previous) = previous_x {
                                assert_eq!(previous - x, strength as u16 + 2, "{case}, tick {tick}");
                            }
                            previous_x = Some(x);
                        }
                    }
                    let current_hp = machine.era_fighters()[0].sub_hp;
                    if current_hp < hp {
                        hits += 1;
                        assert!(!whiff, "{case}, unexpected contact at +{tick}");
                        assert_eq!(tick, release, "{case}");
                        let arcade_hp = if guard { 4 } else { [16, 17, 19][strength - 1] };
                        assert_eq!(hp - current_hp, (arcade_hp * FULL_SUB_HP + 72) / 144, "{case}");
                        assert!(read.connected(Side::P2), "{case}");
                        assert_eq!(read.blocked[1], guard, "{case}");
                        assert_eq!(machine.toggle_state().combo.contact_freeze[..2], [14, 0], "{case}");
                    }
                    if !whiff && tick > release {
                        let ryu = read.fighters[0];
                        let stunned = if guard { ryu.blockstun } else { ryu.hitstun() };
                        assert_eq!(stunned, tick < release + 35, "{case}, tick {tick}");
                    }
                    hp = current_hp;
                    assert_eq!(read.fighters[1].move_id, if tick == ready { 1 } else { 0x1a }, "{case}, tick {tick}");
                }
                assert_eq!(hits, u8::from(!whiff), "{case}");
            }
        }
    }
}

#[test]
fn st_ken_ground_tatsumaki_replays_world_turbo_two() {
    use sf_video::callouts::Side;

    if !Path::new(ROMS).is_dir() {
        return;
    }
    let idle = PlayerInputs::default();
    for loadout in [Loadout::Standard, Loadout::Old] {
        let (assets, base) = native_fight([
            Some((Revision::St, Loadout::Standard)), Some((Revision::St, loadout)),
        ], false);
        for (strength, whiff_end, clean, clean_end, guard_end) in [
            (1, 50, &[1, 39][..], 78, 64),
            (2, 57, &[1, 32][..], 85, 73),
            (3, 64, &[1, 32, 67][..], 106, 80),
        ] {
            for (gap, guard, expected, expected_end) in [
                (240, true, &[][..], whiff_end),
                (55, false, clean, clean_end),
                (55, true, &[1][..], guard_end),
            ] {
                let tag = format!("{loadout:?}, kick {strength}, gap {gap}, guard {guard}");
                let mut fight = base.clone();
                let defense = if guard { pad(6, false) } else { idle };
                for _ in 0..8 {
                    fight.step(&assets, defense, idle, false);
                }
                set_standing_gap(&mut fight, gap);
                if gap == 55 {
                    // Reserve room for two consecutive 78 px recoil paths.
                    let mut pos = fight.save_netplay_snapshot(0);
                    shift_test_fighter(&mut pos, P1_STRUCT, 60);
                    shift_test_fighter(&mut pos, P2_STRUCT, 60);
                    fight.restore_netplay_snapshot(&pos);
                }
                for dir in [4, 5] {
                    fight.step(&assets, defense, pad(dir, false), false);
                }
                let mut kick = pad(1, false);
                kick.buttons[strength + 2] = true;
                fight.step(&assets, defense, kick, false);
                assert_eq!(fight.fight_read().fighters[1].move_id, 0x1b, "{tag}");
                let mut contact = Vec::new();
                let mut health = FULL_SUB_HP;
                let mut first_contact_axis = None;
                let axis = |fight: &Fight, side: Side| {
                    let machine = fight.machine();
                    let telemetry = machine.diagnostic_telemetry();
                    let fighter = if side == Side::P1 { telemetry.p1 } else { telemetry.p2 };
                    let offset = machine.state.objectram[usize::from(fighter.entity_offset) + 0x26] as i8;
                    let camera = u16::from_be_bytes(machine.work_ram()[0x400c..0x400e].try_into().unwrap());
                    fighter.center_x - i32::from(offset) + i32::from(camera)
                };
                let mut floor = None;
                let mut height_max = 0;
                for tick in 1..=expected_end {
                    let saved = (tick == 29 && gap == 55 && strength == 3).then(|| fight.save_netplay_snapshot(tick));
                    fight.step(&assets, defense, idle, false);
                    if let Some(saved) = saved {
                        let checksum = fight.state_checksum();
                        fight.restore_netplay_snapshot(&saved);
                        fight.step(&assets, defense, idle, false);
                        assert_eq!(fight.state_checksum(), checksum, "{tag}");
                    }
                    let read = fight.fight_read();
                    if gap == 240 || guard {
                        let machine = fight.machine();
                        let e = usize::from(machine.diagnostic_telemetry().p2.entity_offset);
                        let o = &machine.state.objectram[e..e + 0x40];
                        let top = i32::from(u16::from_be_bytes([o[4], o[5]]));
                        let cell = if o[2] & 4 == 0 { 16 } else { 32 };
                        let height = sf_engine::data_bank::GROUND - top - (i32::from(o[0xc]) + 1) * cell;
                        height_max = height_max.max(height);
                        if height == 0 && height_max > 0 {
                            floor.get_or_insert(tick);
                        }
                    }
                    if read.connected(Side::P2) {
                        let left = fight.machine().era_fighters()[0].sub_hp;
                        let arcade_hp = if guard { 2 } else { 11 };
                        assert_eq!(health - left, (arcade_hp * FULL_SUB_HP + 72) / 144, "{tag}, hit at {tick}");
                        assert_eq!(read.blocked[1], guard, "{tag}");
                        assert_eq!(fight.machine().toggle_state().combo.contact_freeze[..2], [14, 14], "{tag}");
                        if let Some(hit) = read.landed_by(Side::P2).filter(|hit| !hit.chip) {
                            assert_eq!(hit.into_hitstun, tick == 32, "{tag}, hit at {tick}");
                        }
                        contact.push(tick);
                        first_contact_axis.get_or_insert_with(|| axis(&fight, Side::P1));
                        health = left;
                    }
                    if tick == 36 && (guard && gap == 55 || strength == 1 && !guard) {
                        assert_eq!(first_contact_axis.unwrap() - axis(&fight, Side::P1), 78, "{tag}");
                    }
                    let expected_move = if tick == expected_end { 1 } else { 0x1b };
                    assert_eq!(read.fighters[1].move_id, expected_move, "{tag}, tick {tick}");
                }
                if gap == 240 || guard {
                    let freeze = if gap == 240 { 0 } else { 14 };
                    assert_eq!((height_max, floor), (23, Some(whiff_end - 6 + freeze)), "{tag}");
                }
                assert_eq!(contact, expected, "{tag}");
            }
        }
    }
}

fn walk_ken_into_standing_range(fight: &mut Fight, assets: &Assets, maximum_gap: u16) {
    let neutral = pad(0, false);
    for _ in 0..180 {
        if st_standing_gap(fight) <= maximum_gap {
            break;
        }
        fight.step(assets, neutral, pad(2, false), false);
    }
    for _ in 0..16 {
        fight.step(assets, neutral, neutral, false);
    }
}

#[test]
fn st_ryu_command_normals_use_forward_punch_and_yield_to_close_throws() {
    use sf_engine::combo::throw::THROW_MOVE;
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }

    let neutral = pad(0, false);
    let mut medium = pad(1, false);
    medium.buttons[1] = true;
    let mut heavy = pad(1, false);
    heavy.buttons[2] = true;
    let (assets, mut fight) = native_fight([Some((Revision::St, Loadout::Standard)); 2], false);
    for _ in 0..94 {
        fight.step(&assets, neutral, neutral, false);
    }

    let mut close = fight.clone();
    close_range(&mut close, &assets);
    for walk in [false, true] {
        for attack in [medium, heavy] {
            let mut replay = close.clone();
            if walk {
                replay.step(&assets, pad(1, false), neutral, false);
                assert_eq!(replay.fight_read().fighters[0].move_id, 2);
            }
            replay.step(&assets, attack, neutral, false);
            assert_eq!(replay.fight_read().fighters[0].move_id, THROW_MOVE);
        }
    }

    set_standing_gap(&mut fight, 82);

    for (attack, expected, is_overhead) in [(medium, 0x2a, true), (heavy, 0x2b, false)] {
        let mut replay = fight.clone();
        let snapshot = replay.save_netplay_snapshot(0);
        let mut end = None;
        for pass in 0..2 {
            replay.step(&assets, attack, neutral, false);
            assert_eq!(replay.fight_read().fighters[0].move_id, expected);
            // At the activation gap, the arcade can reach only one hit.
            // Bring Ken into the contact fixture to exercise the full stream.
            set_standing_gap(&mut replay, 56);
            let mut last_health = replay.machine().era_fighters()[1].sub_hp;
            let mut hits = 0;
            for _ in 0..80 {
                replay.step(&assets, neutral, neutral, false);
                let health = replay.machine().era_fighters()[1].sub_hp;
                hits += u8::from(health < last_health);
                last_health = health;
            }
            assert_eq!(hits, 2, "{expected:#04x}");
            if pass == 0 {
                end = Some(replay.state_checksum());
                replay.restore_netplay_snapshot(&snapshot);
            } else {
                assert_eq!(Some(replay.state_checksum()), end);
            }
        }
        for guard in [1, 5] {
            let mut guarded = fight.clone();
            guarded.step(&assets, attack, pad(guard, false), false);
            assert_eq!(guarded.fight_read().fighters[0].move_id, expected);
            set_standing_gap(&mut guarded, 56);
            let mut blocked = false;
            for _ in 0..55 {
                guarded.step(&assets, neutral, pad(guard, false), false);
                blocked |= guarded.fight_read().fighters[1].blockstun;
            }
            let hit = is_overhead && guard == 5;
            assert_eq!(guarded.machine().era_fighters()[1].sub_hp < FULL_SUB_HP, hit);
            assert_eq!(blocked, !hit);
        }
    }

    let (assets, mut old) = native_fight([Some((Revision::St, Loadout::Old)), Some((Revision::St, Loadout::Standard))], false);
    for _ in 0..94 {
        old.step(&assets, neutral, neutral, false);
    }
    for (attack, expected) in [(medium, 0x0f), (heavy, 0x10)] {
        let mut replay = old.clone();
        replay.step(&assets, attack, neutral, false);
        assert_eq!(replay.fight_read().fighters[0].move_id, expected);
    }
}

#[test]
fn st_ryu_standing_lk_changes_guard_height_at_close_range() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }

    let neutral = pad(0, false);
    let mut light_kick = neutral;
    light_kick.buttons[3] = true;
    for (revision, loadout) in [(Revision::St, Loadout::Standard), (Revision::St, Loadout::Old), (Revision::Super, Loadout::Standard)] {
        let (assets, mut far) = native_fight([Some((revision, loadout)), Some((Revision::St, Loadout::Standard))], false);
        for _ in 0..94 {
            far.step(&assets, neutral, neutral, false);
        }
        walk_ken_into_standing_range(&mut far, &assets, 60);
        assert!((49..=61).contains(&st_standing_gap(&far)));
        let mut close = far.clone();
        close_range(&mut close, &assets);
        assert!(st_standing_gap(&close) <= 34);
        if revision != Revision::St {
            // The legacy kick can whiff against Ken's retreating guard pose.
            // Its existing standing selection is the cross-revision control.
            close.step(&assets, light_kick, neutral, false);
            assert_eq!(close.fight_read().fighters[0].move_id, 0x14);
            continue;
        }

        for gap in [32, 40, 41, 60] {
            let at_close_range = gap <= 40;
            for guard in [0, 1, 5] {
                let mut fight = close.clone();
                set_standing_gap(&mut fight, gap);
                fight.step(&assets, light_kick, neutral, false);
                assert_eq!(fight.fight_read().fighters[0].move_id, if at_close_range { 0x2c } else { 0x14 });
                let mut blocked = false;
                for _ in 0..45 {
                    fight.step(&assets, neutral, pad(guard, false), false);
                    blocked |= fight.fight_read().fighters[1].blockstun;
                }
                let hit = guard == 0 || (at_close_range && guard == 1);
                let should_block = if at_close_range { guard == 5 } else { guard == 1 };
                assert_eq!(fight.machine().era_fighters()[1].sub_hp < FULL_SUB_HP, hit, "{loadout:?}, guard {guard}, gap {gap}");
                assert_eq!(blocked, should_block, "{loadout:?}, guard {guard}, gap {gap}");
            }
        }
    }
}

#[test]
fn st_ryu_close_lp_and_mk_use_distinct_standing_streams() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let neutral = pad(0, false);
    let mut medium_kick = neutral;
    medium_kick.buttons[4] = true;
    for loadout in [Loadout::Standard, Loadout::Old] {
        let (assets, mut base) = native_fight([Some((Revision::St, loadout)), Some((Revision::St, Loadout::Standard))], false);
        for _ in 0..94 {
            base.step(&assets, neutral, neutral, false);
        }
        close_range(&mut base, &assets);
        let mut jab_streams = Vec::new();
        for gap in [64, 65] {
            let mut fight = base.clone();
            set_standing_gap(&mut fight, gap);
            fight.step(&assets, pad(0, true), neutral, false);
            let ryu = fight.machine().diagnostic_telemetry().p1;
            assert_eq!(ryu.move_id, 0x0d);
            let pointer = usize::from(ryu.entity_offset) + 0x2c;
            let objects = &fight.machine().state.objectram;
            let stream = u32::from_be_bytes(objects[pointer..pointer + 4].try_into().unwrap());
            assert!(sf_engine::data_bank::in_bank(stream));
            jab_streams.push(stream);
        }
        assert_ne!(jab_streams[0], jab_streams[1]);

        let far_active = if loadout == Loadout::Old { 12 } else { 8 };
        for (gap, expected_move, expected_active) in [(40, 0x2d, 4), (41, 0x15, far_active)] {
            let mut fight = base.clone();
            set_standing_gap(&mut fight, gap);
            fight.step(&assets, medium_kick, neutral, false);
            assert_eq!(fight.fight_read().fighters[0].move_id, expected_move);
            let active = (1..20).find(|_| {
                fight.step(&assets, neutral, neutral, false);
                fight.fight_read().fighters[0].attack_active || fight.machine().era_fighters()[1].sub_hp < FULL_SUB_HP
            });
            assert_eq!(active, Some(expected_active), "{loadout:?} at {gap}px");
            for _ in 0..18 {
                fight.step(&assets, neutral, neutral, false);
            }
            assert!(fight.machine().era_fighters()[1].sub_hp < FULL_SUB_HP, "{loadout:?} MK at {gap}px");
        }
    }
}

#[test]
fn st_ryu_mp_hp_and_hk_use_world_proximity_and_whiff_phases() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let neutral = pad(0, false);
    for loadout in [Loadout::Standard, Loadout::Old] {
        let (assets, mut base) = native_fight([Some((Revision::St, loadout)), Some((Revision::St, Loadout::Standard))], false);
        for _ in 0..94 {
            base.step(&assets, neutral, neutral, false);
        }
        close_range(&mut base, &assets);
        let hk_range = if loadout == Loadout::Old { 79 } else { 95 };
        for (button, boundary, close, far, close_contact, far_contact) in [
            (1, 70, (0x0f, 4..6, 23), (0x0f, 4..8, 15), 4, 4),
            (2, 80, (0x10, 4..12, 35), (0x10, 6..12, 35), 4, 6),
            (5, hk_range, (0x2f, 8..20, 31), (0x16, 3..15, 32), 8, 7),
        ] {
            let mut input = neutral;
            input.buttons[button] = true;
            let mut streams = Vec::new();
            for (gap, (move_id, active_ticks, end_tick)) in [(boundary, close), (boundary + 1, far)] {
                let mut fight = base.clone();
                set_standing_gap(&mut fight, gap);
                fight.step(&assets, input, neutral, false);
                let ryu = fight.machine().diagnostic_telemetry().p1;
                assert_eq!(ryu.move_id, move_id, "{loadout:?} button {button} at {gap}px");
                let pointer = usize::from(ryu.entity_offset) + 0x2c;
                let objects = &fight.machine().state.objectram;
                let stream = u32::from_be_bytes(objects[pointer..pointer + 4].try_into().unwrap());
                assert!(sf_engine::data_bank::in_bank(stream));
                streams.push(stream);

                // Proximity was selected on the ordinary input edge. Move Ken
                // outside the strike now to measure the committed whiff stream.
                set_standing_gap(&mut fight, 144);
                for tick in 1..=end_tick {
                    fight.step(&assets, neutral, neutral, false);
                    let [ryu, _] = fight.fight_read().fighters;
                    assert_eq!(ryu.attack_active, active_ticks.contains(&tick), "{loadout:?} button {button} at {gap}px tick {tick}");
                    assert_eq!(ryu.state & 0x80 != 0, tick < end_tick, "{loadout:?} button {button} at {gap}px tick {tick}");
                }
                assert_eq!(fight.machine().era_fighters()[1].sub_hp, FULL_SUB_HP);
            }
            assert_ne!(streams[0], streams[1]);

            for (gap, expected_contact) in [(40, close_contact), (boundary + 1, far_contact)] {
                let mut fight = base.clone();
                set_standing_gap(&mut fight, gap);
                fight.step(&assets, input, neutral, false);
                let hit = (1..20).find(|_| {
                    fight.step(&assets, neutral, neutral, false);
                    fight.machine().era_fighters()[1].sub_hp < FULL_SUB_HP
                });
                assert_eq!(hit, Some(expected_contact), "{loadout:?} button {button} at {gap}px contacts standing Ken");
                assert_eq!(fight.fight_read().fighters[1].state & 1, 0);
            }
        }
        for (gap, expected_contact) in [(hk_range.min(80), 8), (hk_range, 16)] {
            if loadout == Loadout::Old && expected_contact == 16 {
                continue;
            }
            let mut fight = base.clone();
            set_standing_gap(&mut fight, gap);
            let mut hk = neutral;
            hk.buttons[5] = true;
            fight.step(&assets, hk, neutral, false);
            let contact = (1..25).find(|_| {
                fight.step(&assets, neutral, neutral, false);
                fight.machine().era_fighters()[1].sub_hp < FULL_SUB_HP
            });
            assert_eq!(contact, Some(expected_contact), "{loadout:?} close HK at {gap}px");
        }
    }
}

#[test]
fn st_ryu_standing_hits_use_arcade_contact_clocks_and_recoil() {
    use sf_video::callouts::Side;
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let neutral = pad(0, false);
    let world_x = |fight: &Fight| {
        let machine = fight.machine();
        let ken = machine.diagnostic_telemetry().p2;
        let camera = u16::from_be_bytes(machine.work_ram()[0x400c..0x400e].try_into().unwrap());
        let offset = machine.state.objectram[usize::from(ken.entity_offset) + 0x26] as i8;
        i32::from(camera) + ken.center_x - i32::from(offset)
    };
    for loadout in [Loadout::Standard, Loadout::Old] {
        let (assets, mut base) = native_fight(
            [Some((Revision::St, loadout)), Some((Revision::St, Loadout::Standard))],
            false,
        );
        for _ in 0..94 {
            base.step(&assets, neutral, neutral, false);
        }
        close_range(&mut base, &assets);
        let far_hk = if loadout == Loadout::Old { 80 } else { 96 };
        let cases = [
            (0, 40, 11, 30), (1, 40, 16, 54), (2, 40, 20, 78),
            (3, 40, 11, 30), (4, 40, 16, 54), (5, 32, 20, 78),
            (0, 65, 11, 30), (1, 71, 16, 54), (2, 81, 20, 78),
            (3, 41, 11, 30), (4, 41, 16, 54), (5, far_hk, 20, 78),
        ];
        for (button, gap, stun, push) in cases {
            let mut fight = base.clone();
            set_standing_gap(&mut fight, gap);
            let mut input = neutral;
            input.buttons[button] = true;
            fight.step(&assets, input, neutral, false);
            let first = (1..20).find(|_| {
                fight.step(&assets, neutral, neutral, false);
                fight.fight_read().landed_by(Side::P1).is_some()
            }).expect("standing normal contacts at the measured gap");
            let origin = world_x(&fight);
            assert_eq!(fight.machine().toggle_state().combo.hitstun[1], stun);
            assert_eq!(fight.machine().toggle_state().combo.contact_freeze[..2], [14, 14]);
            for elapsed in 1..=16 + stun {
                fight.step(&assets, neutral, neutral, false);
                let combo = &fight.machine().toggle_state().combo;
                assert_eq!(
                    combo.contact_freeze[..2], [14_u8.saturating_sub(elapsed); 2],
                    "{loadout:?} button {button} gap {gap} at contact +{elapsed}"
                );
                assert!(fight.fight_read().landed_by(Side::P1).is_none(), "{loadout:?} button {button} at {gap}px");
                if elapsed <= 15 {
                    assert_eq!(world_x(&fight), origin);
                }
                if elapsed == 16 + stun {
                    assert_eq!(world_x(&fight) - origin, push, "{loadout:?} button {button} contact {first}");
                    assert!(!fight.fight_read().fighters[1].hitstun());
                }
            }
        }

        let mut fight = base.clone();
        set_standing_gap(&mut fight, 31);
        let mut hk = neutral;
        hk.buttons[5] = true;
        let origin = world_x(&fight);
        let mut hits = Vec::new();
        for tick in 0..75 {
            if tick == 23 {
                let before = fight.save_netplay_snapshot(tick);
                fight.step(&assets, neutral, neutral, false);
                let after = fight.save_netplay_snapshot(tick + 1);
                fight.restore_netplay_snapshot(&before);
                fight.step(&assets, neutral, neutral, false);
                assert!(fight.save_netplay_snapshot(tick + 1) == after);
            } else {
                fight.step(&assets, if tick == 0 { hk } else { neutral }, neutral, false);
            }
            if let Some(hit) = fight.fight_read().landed_by(Side::P1) {
                hits.push((tick, world_x(&fight) - origin, hit.into_hitstun));
                assert_eq!(fight.machine().toggle_state().combo.contact_freeze[..2], [14, 14]);
            }
            match tick {
                44 => assert_eq!(world_x(&fight) - origin, 51),
                45 => assert_eq!(world_x(&fight) - origin, 59),
                64 => assert_eq!(world_x(&fight) - origin, 129),
                65 => assert!(!fight.fight_read().fighters[1].hitstun()),
                _ => {}
            }
        }
        assert_eq!(hits, [(8, 0, false), (30, 51, true)], "{loadout:?} close HK");
        assert_eq!(world_x(&fight) - origin, 129, "{loadout:?} close HK final recoil");

        for (direction, gap, contacts) in [(1, 60, 2), (1, 61, 1), (5, 70, 2), (5, 71, 1)] {
            let mut fight = base.clone();
            set_standing_gap(&mut fight, gap);
            let guard = pad(direction, false);
            let mut blocks = Vec::new();
            for tick in 0..66 {
                fight.step(&assets, if tick == 0 { hk } else { neutral }, if tick == 0 { neutral } else { guard }, false);
                let read = fight.fight_read();
                assert!(read.landed_by(Side::P1).is_none(), "{loadout:?} HK hit guard {direction} at {gap}px");
                if read.blocked[0] {
                    blocks.push(tick);
                    assert_eq!(fight.machine().toggle_state().combo.contact_freeze[..2], [14, 14]);
                }
            }
            assert_eq!(blocks, [8, 30][..contacts], "{loadout:?} HK guard {direction} at {gap}px");
        }
    }
}

#[test]
fn st_ryu_standing_guard_matches_world_contact_matrix() {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return;
    }
    let neutral = pad(0, false);
    let mut mismatches = Vec::new();
    for loadout in [Loadout::Standard, Loadout::Old] {
        let (assets, mut base) = native_fight(
            [Some((Revision::St, loadout)), Some((Revision::St, Loadout::Standard))],
            false,
        );
        for _ in 0..94 {
            base.step(&assets, neutral, neutral, false);
        }
        close_range(&mut base, &assets);
        let far_hk = if loadout == Loadout::Old { 80 } else { 96 };
        for (button, gap, crouch_blocks) in [
            (0, 32, false), (1, 32, true), (2, 32, true), (4, 32, true),
            (0, 65, false), (1, 71, false), (2, 81, false), (4, 41, false),
            (5, far_hk, false),
        ] {
            let mut input = neutral;
            input.buttons[button] = true;
            for (direction, blocks) in [(1, true), (5, crouch_blocks)] {
                let mut fight = base.clone();
                set_standing_gap(&mut fight, gap);
                let guard = pad(direction, false);
                fight.step(&assets, input, neutral, false);
                let mut contacts = 0;
                for _ in 0..50 {
                    fight.step(&assets, neutral, guard, false);
                    if fight.fight_read().blocked[0] {
                        contacts += 1;
                        assert_eq!(fight.machine().toggle_state().combo.contact_freeze[..2], [14, 14]);
                    }
                }
                let hp = fight.machine().era_fighters()[1].sub_hp;
                if contacts != usize::from(blocks) || hp != FULL_SUB_HP {
                    mismatches.push((loadout, button, gap, direction, blocks, contacts, hp));
                }
            }
        }
    }
    assert!(mismatches.is_empty(), "loadout, button, gap, guard, expected block, blocks, HP: {mismatches:?}");
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
        for _ in 0..9 {
            fight.step(&assets, idle, idle, false);
        }
        assert_eq!(fight.machine().era_super_freeze(), ST_RYU_SHINKU_FREEZE - 1);
        fight.step(&assets, idle, pad(0, true), false);
        assert!(!fight.fight_read().fighters[1].attacking());
        for _ in 0..ST_RYU_SHINKU_FREEZE {
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
