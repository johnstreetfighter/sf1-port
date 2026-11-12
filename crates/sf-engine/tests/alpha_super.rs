//! Native Alpha Super Combos through real character selection and pad inputs.

#[path = "support/alpha.rs"]
mod alpha;

use alpha::{local_fight, local_select, local_select_with_p1, pad, DOWN, LEFT, LP, RIGHT};
use sf_engine::data_bank::{ALPHA_SUPER_SECOND_SLOT, ALPHA_SUPER_THIRD_SLOT, SF2_SUPER_SLOT};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::{Fight, FightFrame};
use sf_engine::front::{GameSession, PlayerInputs};
use sf_engine::netplay::{encode_inputs, RollbackEngine};
use sf_video::announcer::AnnouncerLine;
use sf_video::callouts::{CalloutKind, Side};
use sf_video::romset::Assets;

const STOCK: u8 = 48;
const MP: usize = 1;
const HP: usize = 2;
const LK: usize = 3;
const HK: usize = 5;
const QCF_X2: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];
const QCF_DP: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT];
const QCB_X2: &[u8] = &[DOWN, DOWN | LEFT, LEFT, DOWN, DOWN | LEFT, LEFT];

fn bout(select: &GameSession, p2_revision: Revision, p2_loadout: Loadout) -> Fight {
    local_fight(select, p2_revision, p2_loadout)
        .fight()
        .expect("a local fight")
        .clone()
}

fn meter(fight: &Fight, side: usize) -> u8 {
    fight.machine().era_fighters()[side].super_meter
}

fn set_meter(fight: &mut Fight, side: usize, amount: u8) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[side].super_meter = amount;
    fight.restore_netplay_snapshot(&snapshot);
}

fn command_input(motion: &[u8], button: usize, side: usize, frame: usize) -> PlayerInputs {
    let Some(&direction) = motion.get(frame) else {
        return PlayerInputs::default();
    };
    let direction = if side == 0 {
        direction
    } else {
        (direction & DOWN) | ((direction & RIGHT) << 1) | ((direction & LEFT) >> 1)
    };
    pad(direction, (frame + 1 == motion.len()).then_some(button))
}

fn step_with(fight: &mut Fight, assets: &Assets, side: usize, input: PlayerInputs) -> FightFrame {
    let mut pads = [PlayerInputs::default(); 2];
    pads[side] = input;
    fight.step(assets, pads[0], pads[1], false)
}

fn command(
    fight: &mut Fight,
    assets: &Assets,
    side: usize,
    motion: &[u8],
    button: usize,
) -> FightFrame {
    for frame in 0..motion.len() - 1 {
        step_with(
            fight,
            assets,
            side,
            command_input(motion, button, side, frame),
        );
    }
    step_with(
        fight,
        assets,
        side,
        command_input(motion, button, side, motion.len() - 1),
    )
}

fn super_calls(frame: &FightFrame) -> usize {
    frame
        .announcements
        .iter()
        .filter(|&&line| line == AnnouncerLine::Super)
        .count()
}

fn assert_super(fight: &Fight, frame: &FightFrame, side: usize, slot: u8, name: &'static str) {
    assert_eq!(fight.fight_read().fighters[side].move_id, slot);
    assert!(
        fight.machine().era_super_freeze() > 0,
        "{name} freezes the fight"
    );
    assert_eq!(super_calls(frame), 1, "{name} is announced once");
    assert!(
        has_super_callout(fight, side, name),
        "the callout names {name}"
    );
}

fn has_super_callout(fight: &Fight, side: usize, name: &'static str) -> bool {
    fight
        .callouts()
        .frame()
        .side(Side::BOTH[side])
        .words
        .contains(&Some(CalloutKind::Super { name }))
}

fn approach(fight: &mut Fight, assets: &Assets) {
    for _ in 0..100 {
        let read = fight.machine().diagnostic_telemetry();
        if read.p2.center_x - read.p1.center_x <= 38 {
            for _ in 0..8 {
                step_with(fight, assets, 0, PlayerInputs::default());
            }
            return;
        }
        step_with(fight, assets, 0, pad(RIGHT, None));
    }
    panic!("Ryu and Ken did not reach close range");
}

#[test]
fn shinkuu_hadoken_spends_selected_stocks_and_scales_real_projectile_hits() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::Sfz3, Loadout::AIsm)
    else {
        return;
    };
    let base = bout(&select, Revision::Sf1Plus, Loadout::Standard);
    assert_eq!(
        base.machine().era_fighters()[0].style.loadout,
        Loadout::AIsm
    );
    let assets = select.assets();
    let mut previous_damage = 0;
    for (button, level, hit_limit) in [(LP, 1, 2), (MP, 2, 3), (HP, 3, 5)] {
        let mut fight = base.clone();
        set_meter(&mut fight, 0, 3 * STOCK);
        let initial_hp = fight.machine().era_fighters()[1].sub_hp;
        let frame = command(&mut fight, assets, 0, QCF_X2, button);
        assert_super(&fight, &frame, 0, SF2_SUPER_SLOT, "SHINKUU HADOKEN");
        assert_eq!(meter(&fight, 0), (3 - level) * STOCK);

        let mut spawned_hits = 0;
        let mut hit_count = 0;
        let mut hp = initial_hp;
        for _ in 0..135 {
            let output = step_with(&mut fight, assets, 0, PlayerInputs::default());
            assert_eq!(super_calls(&output), 0);
            for projectile in fight.machine().toggle_state().era.projectiles() {
                if projectile.move_id == SF2_SUPER_SLOT && projectile.hits_remaining > 0 {
                    assert_eq!(projectile.alpha_super_level, level);
                    spawned_hits = spawned_hits.max(projectile.hits_remaining);
                }
            }
            let next_hp = fight.machine().era_fighters()[1].sub_hp;
            hit_count += usize::from(next_hp < hp);
            hp = next_hp;
        }
        assert_eq!(
            spawned_hits, hit_limit,
            "Lv{level} shot starts with its own cap"
        );
        assert_eq!(
            hit_count,
            usize::from(hit_limit),
            "Lv{level} lands each hit"
        );
        let damage = initial_hp - hp;
        assert!(
            damage > previous_damage,
            "Lv{level} deals {damage} after {previous_damage}"
        );
        previous_damage = damage;
    }
}

#[test]
fn ken_can_launch_shoryureppa_from_the_original_mirrored_motion() {
    let Some(select) = local_select(GaugeMode::EraDefault) else {
        return;
    };
    let assets = select.assets();
    let mut fight = bout(&select, Revision::Sfz3, Loadout::AIsm);
    approach(&mut fight, assets);
    set_meter(&mut fight, 1, 3 * STOCK);
    let initial_hp = fight.machine().era_fighters()[0].sub_hp;
    let frame = command(&mut fight, assets, 1, QCF_DP, HP);
    assert_super(&fight, &frame, 1, SF2_SUPER_SLOT, "SHORYUREPPA");
    assert_eq!(meter(&fight, 1), 0);
    let mut launches = 0;
    let mut rising = false;
    let mut launched_target = false;
    for _ in 0..150 {
        step_with(&mut fight, assets, 1, PlayerInputs::default());
        let [ryu, ken] = fight.fight_read().fighters;
        let now_rising = ken.move_id == SF2_SUPER_SLOT && ken.state & 3 == 1;
        launches += usize::from(now_rising && !rising);
        rising = now_rising;
        launched_target |= ryu.hitstun() && ryu.state & 3 == 1;
    }
    assert!(launches >= 2, "Ken performs the successive uppercuts");
    assert!(launched_target, "Shoryureppa launches the opponent");
    assert!(fight.machine().era_fighters()[0].sub_hp < initial_hp);
}

#[test]
fn shoto_super_availability_matches_alpha_revision_loadout_and_meter() {
    use GaugeMode::{EraDefault, NoGauges, Unified};
    use Loadout::{AIsm, Standard, VIsm, XIsm};
    use Revision::{Sfz1, Sfz2, Sfz2A, Sfz3};

    let hadoken = (0, QCF_X2, SF2_SUPER_SLOT, "SHINKUU HADOKEN");
    let metsu = (0, QCF_DP, ALPHA_SUPER_SECOND_SLOT, "METSU SHORYUKEN");
    let tatsumaki = (0, QCB_X2, ALPHA_SUPER_THIRD_SLOT, "SHINKUU TATSUMAKI");
    let shoryureppa = (1, QCF_DP, SF2_SUPER_SLOT, "SHORYUREPPA");
    let shinryuken = (1, QCF_X2, ALPHA_SUPER_SECOND_SLOT, "SHINRYUKEN");
    let shippu = (1, QCB_X2, ALPHA_SUPER_THIRD_SLOT, "SHIPPU JINRAIKYAKU");
    let full = 3 * STOCK;

    type SuperMotion = (usize, &'static [u8], u8, &'static str);
    type StockCase = (SuperMotion, usize, u8, u8);
    let fixtures: &[(Revision, Loadout, GaugeMode, &[StockCase])] = &[
        (Sfz1, Standard, EraDefault, &[(hadoken, LP, STOCK, STOCK)]),
        (
            Sfz2,
            Standard,
            EraDefault,
            &[(tatsumaki, LK, 2 * STOCK, STOCK)],
        ),
        (
            Sfz2A,
            Standard,
            EraDefault,
            &[(shinryuken, LK, 2 * STOCK, STOCK), (metsu, LK, full, 0)],
        ),
        (
            Sfz3,
            AIsm,
            EraDefault,
            &[
                (metsu, LK, full, full),
                (shippu, LK, full, full),
                (shippu, LK, full - 1, 0),
                (hadoken, HP, 2 * STOCK, 0),
            ],
        ),
        (
            Sfz3,
            XIsm,
            EraDefault,
            &[
                (hadoken, HP, STOCK, STOCK),
                (shinryuken, LK, STOCK, 0),
                (hadoken, LP, STOCK - 1, 0),
            ],
        ),
        (Sfz3, XIsm, Unified, &[(shoryureppa, LP, full, full)]),
        (Sfz3, VIsm, EraDefault, &[(hadoken, LP, 2 * STOCK, 0)]),
        (Sfz3, AIsm, NoGauges, &[(hadoken, LP, 0, 0)]),
    ];

    for &(revision, loadout, gauge_mode, cases) in fixtures {
        let Some(select) = local_select_with_p1(gauge_mode, revision, loadout) else {
            return;
        };
        let base = bout(&select, revision, loadout);
        let assets = select.assets();
        for &((side, motion, slot, name), button, stocked, spent) in cases {
            let mut fight = base.clone();
            set_meter(&mut fight, side, stocked);
            let frame = command(&mut fight, assets, side, motion, button);
            if spent > 0 {
                assert_super(&fight, &frame, side, slot, name);
                assert_eq!(meter(&fight, side), stocked - spent);
            } else {
                assert_eq!(
                    super_calls(&frame),
                    0,
                    "{name} with {revision:?} {loadout:?} {gauge_mode:?} at {stocked} meter"
                );
                assert_ne!(fight.fight_read().fighters[side].move_id, slot);
                assert!(meter(&fight, side) >= stocked);
            }
        }
    }
}

#[test]
fn x_ism_changes_real_outgoing_and_incoming_damage_in_sub_health_units() {
    let Some(a_ism) = local_select_with_p1(GaugeMode::EraDefault, Revision::Sfz3, Loadout::AIsm)
    else {
        return;
    };
    let x_ism = local_select_with_p1(GaugeMode::EraDefault, Revision::Sfz3, Loadout::XIsm)
        .expect("the ROM set remains available");
    let damage = |select: &GameSession, p2_loadout| {
        let mut fight = bout(select, Revision::Sfz3, p2_loadout);
        let assets = select.assets();
        approach(&mut fight, assets);
        let hp = fight.machine().era_fighters()[1].sub_hp;
        step_with(&mut fight, assets, 0, pad(DOWN, Some(LP)));
        for _ in 0..35 {
            step_with(&mut fight, assets, 0, PlayerInputs::default());
        }
        let damage = hp - fight.machine().era_fighters()[1].sub_hp;
        assert!(damage > 0, "Ryu lands a clean crouching jab");
        u32::from(damage)
    };
    let baseline = damage(&a_ism, Loadout::AIsm);
    assert_eq!(damage(&x_ism, Loadout::AIsm), (112 * baseline + 50) / 100);
    assert_eq!(damage(&a_ism, Loadout::XIsm), (115 * baseline + 50) / 100);
}

#[test]
fn late_remote_input_replays_named_super_projectile_state_and_damage() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::Sfz3, Loadout::AIsm)
    else {
        return;
    };
    let assets = select.assets();
    let mut reference = bout(&select, Revision::Sf1Plus, Loadout::Standard);
    set_meter(&mut reference, 0, 3 * STOCK);
    let initial_hp = reference.machine().era_fighters()[1].sub_hp;
    let mut predicted = reference.clone();
    let mut net = RollbackEngine::new(1, 0, 8);
    const DELAY: u32 = 4;
    const FRAMES: u32 = 100;
    let remote = |frame: u32| command_input(QCF_X2, MP, 0, frame as usize);
    let mut expected_calls = 0;
    let mut actual_calls = 0;
    let mut projectile_checksum = None;

    for tick in 0..FRAMES + DELAY {
        if let Some(frame) = tick.checked_sub(DELAY) {
            net.apply_remote_inputs(
                &mut predicted,
                assets,
                frame,
                &[encode_inputs(remote(frame))],
            );
            if frame as usize + 1 == QCF_X2.len() {
                assert_eq!(net.last_rollback_frames(), DELAY as u8);
                assert_eq!(meter(&predicted, 0), STOCK);
                assert!(has_super_callout(&predicted, 0, "SHINKUU HADOKEN"));
            }
        }
        if tick < FRAMES {
            let output = step_with(&mut reference, assets, 0, remote(tick));
            expected_calls += super_calls(&output);
            let output = net
                .step(&mut predicted, assets, PlayerInputs::default())
                .expect("under the rollback limit");
            actual_calls += super_calls(&output);
            if tick as usize + 1 == QCF_X2.len() {
                assert_eq!(meter(&reference, 0), STOCK);
                assert_eq!(meter(&predicted, 0), 3 * STOCK);
            }
            if projectile_checksum.is_none()
                && reference
                    .machine()
                    .toggle_state()
                    .era
                    .projectiles()
                    .iter()
                    .any(|projectile| {
                        projectile.alpha_super_level == 2 && projectile.hits_remaining > 0
                    })
            {
                projectile_checksum = Some((tick, reference.state_checksum()));
            }
        }
    }
    let (frame, checksum) = projectile_checksum.expect("the Lv2 projectile was live in flight");
    assert_eq!(net.confirmed_checksum(frame), Some(checksum));
    assert_eq!(net.confirmed_inputs().len(), FRAMES as usize);
    assert_eq!(predicted.state_checksum(), reference.state_checksum());
    assert!(reference.machine().era_fighters()[1].sub_hp < initial_hp);
    assert_eq!((expected_calls, actual_calls), (1, 1));
}

#[test]
fn authored_alpha_melee_supers_land_repeated_hits_and_keep_their_poses_above_the_floor() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::Sfz3, Loadout::AIsm)
    else {
        return;
    };
    let assets = select.assets();
    let mut base = bout(&select, Revision::Sfz3, Loadout::AIsm);
    approach(&mut base, assets);
    for (side, slot, motion, name) in [
        (0, ALPHA_SUPER_SECOND_SLOT, QCF_DP, "METSU SHORYUKEN"),
        (0, ALPHA_SUPER_THIRD_SLOT, QCB_X2, "SHINKUU TATSUMAKI"),
        (1, ALPHA_SUPER_SECOND_SLOT, QCF_X2, "SHINRYUKEN"),
        (1, ALPHA_SUPER_THIRD_SLOT, QCB_X2, "SHIPPU JINRAIKYAKU"),
    ] {
        let mut fight = base.clone();
        set_meter(&mut fight, side, 3 * STOCK);
        let frame = command(&mut fight, assets, side, motion, HK);
        assert_super(&fight, &frame, side, slot, name);
        let mut hp = fight.machine().era_fighters()[1 - side].sub_hp;
        let mut hits = 0;
        let mut airborne = false;
        let mut finished = false;
        for tick in 0..150 {
            step_with(&mut fight, assets, side, PlayerInputs::default());
            let machine = fight.machine();
            let read = machine.diagnostic_telemetry();
            let actor = [read.p1, read.p2][side];
            if actor.move_id != slot {
                finished = true;
                break;
            }
            let next_hp = machine.era_fighters()[1 - side].sub_hp;
            hits += usize::from(next_hp < hp);
            hp = next_hp;
            airborne |= actor.state_byte & 3 == 1;
            let objects = &machine.state.objectram;
            let e = usize::from(actor.entity_offset);
            let attr = u16::from_be_bytes([objects[e + 2], objects[e + 3]]);
            let y = i32::from(i16::from_be_bytes([objects[e + 4], objects[e + 5]]));
            let cell = if attr & 0x400 != 0 { 32 } else { 16 };
            let bottom = y + cell * (i32::from(objects[e + 0xc]) + 1);
            assert!(
                bottom <= sf_engine::data_bank::GROUND,
                "{name} tick {tick} grid bottom {bottom}"
            );
        }
        assert!(airborne && finished, "{name} takes off and returns to idle");
        assert!((2..=5).contains(&hits), "{name} landed {hits} hits");
    }
}

#[test]
fn shoryureppa_scales_by_level() {
    let Some(select) = local_select(GaugeMode::EraDefault) else {
        return;
    };
    let assets = select.assets();
    let mut base = bout(&select, Revision::Sfz3, Loadout::AIsm);
    approach(&mut base, assets);
    let mut previous = 0;
    for (level, button, expected_hits) in [(1, LP, 2), (2, MP, 3), (3, HP, 4)] {
        let mut fight = base.clone();
        set_meter(&mut fight, 1, 3 * STOCK);
        command(&mut fight, assets, 1, QCF_DP, button);
        let initial_hp = fight.machine().era_fighters()[0].sub_hp;
        let mut hp = initial_hp;
        let mut hits = 0;
        for _ in 0..150 {
            step_with(&mut fight, assets, 1, PlayerInputs::default());
            let next_hp = fight.machine().era_fighters()[0].sub_hp;
            hits += usize::from(next_hp < hp);
            hp = next_hp;
        }
        let damage = initial_hp - hp;
        assert_eq!(hits, expected_hits, "Shoryureppa Lv{level} hit count");
        assert!(
            damage > previous,
            "Lv{level} damage must exceed Lv{} damage",
            level - 1
        );
        previous = damage;
    }
}
