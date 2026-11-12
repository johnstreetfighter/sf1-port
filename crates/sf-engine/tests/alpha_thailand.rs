//! Sagat and Adon's Alpha moves through the native fight and banked art.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod alpha;

use alpha::{DOWN, LEFT, RIGHT, UP, local_fight, local_select_with_fighter, pad};
use sf_engine::data_bank::{
    ALPHA_SUPER_SECOND_SLOT, ALPHA_SUPER_THIRD_SLOT, GROUND, SF2_SUPER_SLOT,
};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::{Fight, FightFrame};
use sf_engine::front::{GameSession, PlayerInputs};
use sf_engine::roster::TournamentFighter;
use sf_video::callouts::{CalloutKind, Side};
use sf_video::romset::Assets;

const HP: usize = 2;
const LK: usize = 3;
const HK: usize = 5;
const FULL_A_ISM: u8 = 144;
const STOCK: u8 = 48;
const DP: &[u8] = &[RIGHT, DOWN, DOWN | RIGHT];
const RDP: &[u8] = &[LEFT, DOWN, DOWN | LEFT];
const TIGER_KNEE: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, UP | RIGHT];
const HALF_BACK: &[u8] = &[RIGHT, DOWN | RIGHT, DOWN, DOWN | LEFT, LEFT];
const QCF_X2: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];
const QCF_DP: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT];
const QCB_X2: &[u8] = &[DOWN, DOWN | LEFT, LEFT, DOWN, DOWN | LEFT, LEFT];

fn select(fighter: TournamentFighter, loadout: Loadout) -> Option<GameSession> {
    local_select_with_fighter(GaugeMode::EraDefault, fighter, Revision::Sfz3, loadout)
}

fn bout(select: &GameSession) -> Fight {
    local_fight(select, Revision::Sf1Plus, Loadout::Standard)
        .fight()
        .expect("a local fight")
        .clone()
}

fn step(fight: &mut Fight, assets: &Assets, input: PlayerInputs) -> FightFrame {
    fight.step(assets, input, PlayerInputs::default(), false)
}

fn command(fight: &mut Fight, assets: &Assets, motion: &[u8], button: usize) {
    for (i, &direction) in motion.iter().enumerate() {
        step(
            fight,
            assets,
            pad(direction, (i + 1 == motion.len()).then_some(button)),
        );
    }
}

fn set_meter(fight: &mut Fight, amount: u8) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[0].super_meter = amount;
    fight.restore_netplay_snapshot(&snapshot);
}

fn approach(fight: &mut Fight, assets: &Assets) {
    for _ in 0..100 {
        let read = fight.machine().diagnostic_telemetry();
        if read.p2.center_x.abs_diff(read.p1.center_x) <= 60 {
            for _ in 0..10 {
                step(fight, assets, PlayerInputs::default());
            }
            return;
        }
        step(fight, assets, pad(RIGHT, None));
    }
    panic!("fighters did not reach close range");
}

fn assert_floor(fight: &Fight, slot: u8, tick: usize) {
    let machine = fight.machine();
    let p1 = machine.diagnostic_telemetry().p1;
    let entity = usize::from(p1.entity_offset);
    let objects = &machine.state.objectram;
    let attr = u16::from_be_bytes([objects[entity + 2], objects[entity + 3]]);
    let top = i32::from(i16::from_be_bytes([
        objects[entity + 4],
        objects[entity + 5],
    ]));
    let cell = if attr & 0x400 != 0 { 32 } else { 16 };
    let bottom = top + cell * (i32::from(objects[entity + 0xc]) + 1);
    assert!(
        bottom <= GROUND,
        "move {slot:#x} tick {tick} sprite bottom {bottom} exceeds the floor"
    );
}

fn assert_super(fight: &Fight, slot: u8, name: &'static str) {
    assert_eq!(fight.fight_read().fighters[0].move_id, slot, "{name}");
    assert!(
        fight.machine().era_super_freeze() > 0,
        "{name} starts a Super freeze"
    );
    assert!(
        fight
            .callouts()
            .frame()
            .side(Side::BOTH[0])
            .words
            .contains(&Some(CalloutKind::Super { name }))
    );
}

#[test]
fn sagat_and_adon_alpha_melee_supers_land_multiple_hits_and_return_to_the_floor() {
    type SuperCase = (&'static [u8], usize, u8, &'static str);
    let cases: [(TournamentFighter, &[SuperCase]); 2] = [
        (
            TournamentFighter::Sagat,
            &[
                (QCF_DP, HK, SF2_SUPER_SLOT, "TIGER GENOCIDE"),
                (QCB_X2, HK, ALPHA_SUPER_THIRD_SLOT, "TIGER RAID"),
            ],
        ),
        (
            TournamentFighter::Adon,
            &[
                (QCF_DP, HP, SF2_SUPER_SLOT, "JAGUAR VARIED ASSAULT"),
                (QCB_X2, HK, ALPHA_SUPER_SECOND_SLOT, "JAGUAR REVOLVER"),
            ],
        ),
    ];
    for (fighter, supers) in cases {
        let Some(select) = select(fighter, Loadout::AIsm) else {
            return;
        };
        let assets = select.assets();
        let mut base = bout(&select);
        approach(&mut base, assets);
        for &(motion, button, slot, name) in supers {
            let mut fight = base.clone();
            set_meter(&mut fight, FULL_A_ISM);
            command(&mut fight, assets, motion, button);
            assert_super(&fight, slot, name);
            assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
            let mut hp = fight.machine().era_fighters()[1].sub_hp;
            let mut hits = 0;
            let mut airborne = false;
            let mut finished = false;
            for tick in 0..160 {
                step(&mut fight, assets, PlayerInputs::default());
                let actor = fight.fight_read().fighters[0];
                if actor.move_id != slot {
                    finished = true;
                    break;
                }
                assert_floor(&fight, slot, tick);
                airborne |= actor.state & 3 == 1;
                let next_hp = fight.machine().era_fighters()[1].sub_hp;
                hits += usize::from(next_hp < hp);
                hp = next_hp;
            }
            assert!(airborne && finished, "{name} takes off and recovers");
            assert!(hits >= 2, "{name} landed {hits} hits");
        }
    }
}

#[test]
fn tiger_cannon_uses_super_projectile_hits_and_respects_x_ism() {
    let Some(a_ism) = select(TournamentFighter::Sagat, Loadout::AIsm) else {
        return;
    };
    let mut fight = bout(&a_ism);
    let assets = a_ism.assets();
    set_meter(&mut fight, FULL_A_ISM);
    command(&mut fight, assets, QCF_X2, HP);
    assert_super(&fight, ALPHA_SUPER_SECOND_SLOT, "TIGER CANNON");
    let mut hp = fight.machine().era_fighters()[1].sub_hp;
    let mut spawned_hits = 0;
    let mut hit_count = 0;
    for _ in 0..160 {
        step(&mut fight, assets, PlayerInputs::default());
        for projectile in fight.machine().toggle_state().era.projectiles() {
            if projectile.move_id == ALPHA_SUPER_SECOND_SLOT && projectile.hits_remaining != 0 {
                assert_eq!(projectile.alpha_super_level, 3);
                spawned_hits = spawned_hits.max(projectile.hits_remaining);
            }
        }
        let next_hp = fight.machine().era_fighters()[1].sub_hp;
        hit_count += usize::from(next_hp < hp);
        hp = next_hp;
    }
    assert_eq!(
        (spawned_hits, hit_count),
        (5, 5),
        "defender finished with {hp} sub-HP"
    );

    let x_ism = select(TournamentFighter::Sagat, Loadout::XIsm).expect("same local ROM set");
    let mut fight = bout(&x_ism);
    set_meter(&mut fight, STOCK);
    command(&mut fight, x_ism.assets(), QCF_X2, HP);
    assert_ne!(
        fight.fight_read().fighters[0].move_id,
        ALPHA_SUPER_SECOND_SLOT
    );
    let mut fight = bout(&x_ism);
    set_meter(&mut fight, STOCK);
    command(&mut fight, x_ism.assets(), QCF_DP, LK);
    assert_super(&fight, SF2_SUPER_SLOT, "TIGER GENOCIDE");
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
}

#[test]
fn angry_charge_spends_one_stock_and_powers_the_next_tiger_blow() {
    let Some(select) = select(TournamentFighter::Sagat, Loadout::AIsm) else {
        return;
    };
    let assets = select.assets();
    let mut base = bout(&select);
    approach(&mut base, assets);
    set_meter(&mut base, 2 * STOCK);
    let tiger_blow_damage = |mut fight: Fight| {
        let hp = fight.machine().era_fighters()[1].sub_hp;
        command(&mut fight, assets, DP, HP);
        assert_eq!(fight.fight_read().fighters[0].move_id, 0x1f);
        for _ in 0..75 {
            step(&mut fight, assets, PlayerInputs::default());
        }
        hp - fight.machine().era_fighters()[1].sub_hp
    };
    let baseline = tiger_blow_damage(base.clone());
    assert!(baseline > 0, "Tiger Blow connects at close range");
    let mut fight = base;
    for direction in [DOWN, DOWN | RIGHT] {
        step(&mut fight, assets, pad(direction, None));
    }
    let mut charge = pad(RIGHT, None);
    charge.buttons[..3].fill(true);
    step(&mut fight, assets, charge);
    assert_eq!(fight.fight_read().fighters[0].move_id, 0x2f);
    assert_eq!(fight.machine().era_fighters()[0].super_meter, STOCK);
    for _ in 0..65 {
        step(&mut fight, assets, PlayerInputs::default());
    }
    assert_eq!(tiger_blow_damage(fight), baseline + baseline / 2);
}

#[test]
fn alpha_special_commands_and_jaguar_tooth_wall_rebound() {
    for (fighter, ism, motion, button, slot) in [
        (TournamentFighter::Sagat, Loadout::AIsm, DP, HP, 0x1f),
        (TournamentFighter::Sagat, Loadout::AIsm, DP, LK, 0x19),
        (
            TournamentFighter::Sagat,
            Loadout::XIsm,
            TIGER_KNEE,
            LK,
            0x19,
        ),
        (TournamentFighter::Adon, Loadout::AIsm, TIGER_KNEE, LK, 0x19),
        (TournamentFighter::Adon, Loadout::AIsm, RDP, LK, 0x1b),
    ] {
        let Some(select) = select(fighter, ism) else {
            return;
        };
        let mut fight = bout(&select);
        command(&mut fight, select.assets(), motion, button);
        assert_eq!(
            fight.fight_read().fighters[0].move_id,
            slot,
            "{fighter:?} {ism:?} {motion:?}"
        );
    }

    let select = select(TournamentFighter::Adon, Loadout::AIsm).expect("same local ROM set");
    let mut fight = bout(&select);
    let assets = select.assets();
    command(&mut fight, assets, HALF_BACK, LK);
    let slot = 0x2c;
    let start = fight.machine().diagnostic_telemetry().p1.center_x;
    assert_eq!(fight.fight_read().fighters[0].move_id, slot);
    let mut furthest_back = start;
    let mut furthest_forward_after_bounce = start;
    let mut finished = false;
    for tick in 0..150 {
        step(&mut fight, assets, PlayerInputs::default());
        let actor = fight.machine().diagnostic_telemetry().p1;
        if actor.move_id != slot {
            finished = true;
            break;
        }
        assert_floor(&fight, slot, tick);
        furthest_back = furthest_back.min(actor.center_x);
        if furthest_back < start {
            furthest_forward_after_bounce = furthest_forward_after_bounce.max(actor.center_x);
        }
    }
    assert!(
        finished && furthest_back + 15 < start && furthest_forward_after_bounce > start + 30,
        "wall rebound from {start} back to {furthest_back} forward to {furthest_forward_after_bounce}, finished {finished}"
    );
}

#[test]
fn x_ism_jaguar_kick_starts_in_the_air_and_recovers_without_crossing_the_floor() {
    let Some(select) = select(TournamentFighter::Adon, Loadout::XIsm) else {
        return;
    };
    let assets = select.assets();
    let mut grounded = bout(&select);
    command(&mut grounded, assets, RDP, LK);
    assert_ne!(grounded.fight_read().fighters[0].move_id, 0x1b);

    for delay in [8, 15] {
        let mut fight = bout(&select);
        step(&mut fight, assets, pad(UP, None));
        for _ in 0..delay {
            step(&mut fight, assets, PlayerInputs::default());
        }
        let jump = fight.fight_read().fighters[0];
        step(&mut fight, assets, pad(0, Some(HK)));
        let mut started = false;
        let mut finished = false;
        for tick in 0..70 {
            let actor = fight.fight_read().fighters[0];
            if actor.move_id == 0x2d {
                assert_floor(&fight, 0x2d, tick);
                started = true;
            } else if started {
                finished = true;
                break;
            }
            step(&mut fight, assets, PlayerInputs::default());
        }
        assert!(
            started && finished,
            "X-ISM Jaguar Kick started {started} and recovered {finished} after {delay} jump frames (move {:#x}, state {:#x})",
            jump.move_id,
            jump.state,
        );
    }
}
