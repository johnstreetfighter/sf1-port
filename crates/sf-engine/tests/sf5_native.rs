//! Street Fighter V through the actual Style Card, input and fighter loop.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod alpha;

use alpha::{
    local_fight, local_select_with_fighter, pad, training_fight_with_fighter, DOWN, LEFT, RIGHT,
};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::{Fight, FightFrame};
use sf_engine::front::{GameSession, PlayerInputs};
use sf_engine::netplay::{encode_inputs, RollbackEngine};
use sf_engine::roster::TournamentFighter::{self, Birdie, Ryu, Sagat};
use sf_video::announcer::AnnouncerLine;
use sf_video::era_hud::{EraGauge, Fill, GaugeRow, GaugeState};
use sf_video::romset::Assets;

const LP: usize = 0;
const MP: usize = 1;
const HP: usize = 2;
const MK: usize = 4;
const HK: usize = 5;
const STOCK: u8 = 48;
const QCF: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT];
const QCF_X2: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];

fn bout(
    fighter: TournamentFighter,
    revision: Revision,
    loadout: Loadout,
) -> Option<(GameSession, Fight)> {
    let select = local_select_with_fighter(GaugeMode::EraDefault, fighter, revision, loadout)?;
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let fight = session.fight().expect("local fight").clone();
    assert_eq!(fight.machine().era_fighters()[0].style.revision, revision);
    assert_eq!(fight.machine().era_fighters()[0].style.loadout, loadout);
    Some((session, fight))
}

fn input(direction: u8, buttons: &[usize]) -> PlayerInputs {
    let mut input = pad(direction, None);
    for &button in buttons {
        input.buttons[button] = true;
    }
    input
}

fn step(fight: &mut Fight, assets: &Assets, p1: PlayerInputs) -> FightFrame {
    let frame = fight.step(assets, p1, PlayerInputs::default(), false);
    assert!(frame.event.is_none(), "fight flow: {:?}", frame.event);
    frame
}

fn wait(fight: &mut Fight, assets: &Assets, ticks: usize) {
    for _ in 0..ticks {
        step(fight, assets, PlayerInputs::default());
    }
}

fn set_ex(fight: &mut Fight, value: u8) {
    let mut snapshot = fight.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[0].super_meter = value;
    fight.restore_netplay_snapshot(&snapshot);
}

fn move_id(fight: &Fight) -> u8 {
    fight.fight_read().fighters[0].move_id
}

fn v_gauge(fight: &Fight) -> GaugeRow {
    fight.machine().era_fighters()[0]
        .gauge()
        .unwrap()
        .second
        .unwrap()
}

fn command(fight: &mut Fight, assets: &Assets, motion: &[u8], buttons: &[usize]) -> FightFrame {
    let (&last, rest) = motion.split_last().unwrap();
    for &direction in rest {
        step(fight, assets, pad(direction, None));
    }
    step(fight, assets, input(last, buttons))
}

fn fill_with_break_time(fight: &mut Fight, assets: &Assets) {
    for _ in 0..6 {
        step(fight, assets, input(0, &[MP, MK]));
        assert_eq!(move_id(fight), 0x2b);
        wait(fight, assets, 45);
    }
    assert_eq!(v_gauge(fight), GaugeRow::stock(2, 2, Fill::EMPTY));
}

#[test]
fn ex_and_critical_art_spend_the_same_three_stock_gauge() {
    let Some((session, mut fight)) = bout(Ryu, Revision::Sf5Ce, Loadout::Skill1Trigger1) else {
        return;
    };
    let assets = session.assets();
    let mut empty = fight.clone();
    let empty_stock = Fill::of(0, STOCK.into());
    assert_eq!(
        empty.era_hud().p1,
        Some(EraGauge::two(
            GaugeRow::stock(3, 0, empty_stock),
            GaugeRow::stock(2, 0, empty_stock)
        ))
    );
    command(&mut empty, assets, QCF, &[LP, MP]);
    assert_ne!(move_id(&empty), 0x20, "EX requires one stock");
    wait(&mut empty, assets, 100);
    let art = command(&mut empty, assets, QCF_X2, &[HP]);
    assert_ne!(move_id(&empty), 0x29, "Critical Art requires three stocks");
    assert!(!art.announcements.contains(&AnnouncerLine::Super));

    set_ex(&mut fight, 3 * STOCK);
    command(&mut fight, assets, QCF, &[LP, MP]);
    assert_eq!(move_id(&fight), 0x20, "EX Hadoken");
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 2 * STOCK);
    wait(&mut fight, assets, 100);

    set_ex(&mut fight, 3 * STOCK);
    let art = command(&mut fight, assets, QCF_X2, &[HP]);
    assert_eq!(move_id(&fight), 0x29, "Shinku Hadoken");
    assert!(art.announcements.contains(&AnnouncerLine::Super));
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
    wait(&mut fight, assets, 100);
    assert!(
        fight.machine().era_fighters()[1].sub_hp < 960,
        "the native projectile lands"
    );
}

#[test]
fn break_time_builds_v_and_enjoy_time_drains_it_without_spending_ex() {
    let Some((session, mut fight)) = bout(Birdie, Revision::Sf5Ce, Loadout::Skill1Trigger1) else {
        return;
    };
    let assets = session.assets();
    fill_with_break_time(&mut fight, assets);
    set_ex(&mut fight, STOCK);
    step(&mut fight, assets, input(0, &[HP, HK]));
    assert_eq!(move_id(&fight), 0x2a);
    assert!(matches!(
        v_gauge(&fight),
        GaugeRow::Stock {
            state: GaugeState::Draining,
            ..
        }
    ));
    assert_eq!(fight.machine().era_fighters()[0].super_meter, STOCK);
    let active = v_gauge(&fight);
    wait(&mut fight, assets, 120);
    assert_ne!(active, v_gauge(&fight), "the V-Timer drains");
    assert_eq!(fight.machine().era_fighters()[0].super_meter, STOCK);
}

#[test]
fn whiffed_v_shift_spends_a_stock_and_does_not_open_a_break() {
    let Some((session, mut fight)) = bout(Birdie, Revision::Sf5Ce, Loadout::Skill1Trigger1) else {
        return;
    };
    let assets = session.assets();
    fill_with_break_time(&mut fight, assets);
    step(&mut fight, assets, input(0, &[HP, MK]));
    assert_eq!(move_id(&fight), 0x2e);
    assert_eq!(
        v_gauge(&fight),
        GaugeRow::stock(2, 1, Fill::of(0, STOCK.into()))
    );
    for _ in 0..30 {
        step(&mut fight, assets, input(0, &[HP, MK]));
        assert_ne!(move_id(&fight), 0x2f);
    }
    assert_eq!(
        v_gauge(&fight),
        GaugeRow::stock(2, 1, Fill::of(0, STOCK.into()))
    );
}

#[test]
fn ae_sagat_uses_the_style_card_for_his_revision_and_real_uppercut() {
    let Some((session, mut fight)) = bout(Sagat, Revision::Sf5Ae, Loadout::Skill1Trigger2) else {
        return;
    };
    assert_eq!(
        v_gauge(&fight),
        GaugeRow::stock(3, 0, Fill::of(0, STOCK.into()))
    );
    let assets = session.assets();
    step(&mut fight, assets, input(0, &[MP, MK]));
    assert_eq!(move_id(&fight), 0x2b);
    wait(&mut fight, assets, 45);
    command(&mut fight, assets, &[RIGHT, DOWN, DOWN | RIGHT], &[HP]);
    assert_eq!(move_id(&fight), 0x1f, "Tiger Uppercut");
}

#[test]
fn arcade_edition_cannot_v_shift_even_with_a_full_v_gauge() {
    let Some((session, mut fight)) = bout(Birdie, Revision::Sf5Ae, Loadout::Skill1Trigger1) else {
        return;
    };
    let assets = session.assets();
    fill_with_break_time(&mut fight, assets);
    let before = v_gauge(&fight);
    step(&mut fight, assets, input(0, &[HP, MK]));
    assert_ne!(move_id(&fight), 0x2e);
    assert_eq!(v_gauge(&fight), before);
}

#[test]
fn champion_edition_ryu_uses_thrust_strike_and_kakko_fubatsu() {
    let Some(session) = training_fight_with_fighter(Ryu, Revision::Sf5Ce, Loadout::Skill2Trigger2)
    else {
        return;
    };
    let assets = session.assets();
    let mut fight = session.fight().unwrap().clone();
    wait(&mut fight, assets, 5);
    assert_eq!(v_gauge(&fight), GaugeRow::stock(3, 3, Fill::EMPTY));

    step(&mut fight, assets, input(0, &[MP, MK]));
    assert_eq!(move_id(&fight), 0x2c, "Thrust Strike");
    wait(&mut fight, assets, 65);
    step(&mut fight, assets, input(0, &[HP, HK]));
    assert_eq!(move_id(&fight), 0x2a, "Kakko Fubatsu");
    assert!(matches!(
        v_gauge(&fight),
        GaugeRow::Stock {
            cells: 3,
            state: GaugeState::Draining,
            ..
        }
    ));
    wait(&mut fight, assets, 65);
    step(&mut fight, assets, input(0, &[HP, HK]));
    assert_eq!(
        move_id(&fight),
        0x24,
        "Isshin is available during the install"
    );
}

#[test]
fn delayed_shift_input_replays_the_v_gauge_and_fighter_state() {
    let Some((session, mut reference)) = bout(Birdie, Revision::Sf5Ce, Loadout::Skill1Trigger1)
    else {
        return;
    };
    let assets = session.assets();
    fill_with_break_time(&mut reference, assets);
    let mut predicted = reference.clone();
    let mut rollback = RollbackEngine::new(1, 0, 8);
    let remote = |tick| {
        if tick == 4 {
            input(0, &[HP, MK])
        } else {
            PlayerInputs::default()
        }
    };
    for tick in 0u32..39 {
        if let Some(frame) = tick.checked_sub(4) {
            rollback.apply_remote_inputs(
                &mut predicted,
                assets,
                frame,
                &[encode_inputs(remote(frame))],
            );
        }
        if tick < 35 {
            step(&mut reference, assets, remote(tick));
            rollback
                .step(&mut predicted, assets, PlayerInputs::default())
                .expect("within rollback window");
        }
    }
    assert_eq!(predicted.state_checksum(), reference.state_checksum());
    assert_eq!(
        v_gauge(&predicted),
        GaugeRow::stock(2, 1, Fill::of(0, STOCK.into()))
    );
}

#[test]
fn v_shift_evading_a_real_strike_refunds_half_a_bar_and_allows_break() {
    let Some((session, mut base)) = bout(Birdie, Revision::Sf5Ce, Loadout::Skill1Trigger1) else {
        return;
    };
    let assets = session.assets();
    fill_with_break_time(&mut base, assets);
    for _ in 0..38 {
        base.step(assets, pad(RIGHT, None), pad(LEFT, None), false);
    }
    wait(&mut base, assets, 4);
    let mut attacked = false;
    for button in [LP, HP, HK] {
        let initial_hp = base.machine().era_fighters()[0].sub_hp;
        let mut baseline = base.clone();
        let Some(contact) = (0usize..24).find(|&tick| {
            let p2 = if tick == 0 {
                input(0, &[button])
            } else {
                PlayerInputs::default()
            };
            baseline.step(assets, PlayerInputs::default(), p2, false);
            baseline.machine().era_fighters()[0].sub_hp < initial_hp
        }) else {
            continue;
        };
        attacked = true;
        for timing in contact.saturating_sub(3)..=contact {
            let mut fight = base.clone();
            let mut saw_shift_break = false;
            for tick in 0..35 {
                let p1 = if tick >= timing {
                    input(0, &[HP, MK])
                } else {
                    PlayerInputs::default()
                };
                let p2 = if tick == 0 {
                    input(0, &[button])
                } else {
                    PlayerInputs::default()
                };
                fight.step(assets, p1, p2, false);
                saw_shift_break |= move_id(&fight) == 0x2f;
            }
            if saw_shift_break {
                assert_eq!(fight.machine().era_fighters()[0].sub_hp, initial_hp);
                assert_eq!(
                    v_gauge(&fight),
                    GaugeRow::stock(2, 1, Fill::of(u16::from(STOCK / 2), STOCK.into()))
                );
                return;
            }
        }
    }
    assert!(attacked, "the test reaches a real strike in a control bout");
    panic!("no timing evaded the nearby strike");
}

#[test]
fn v_reversal_requires_real_blockstun_spends_v_and_deals_recoverable_damage() {
    let Some((session, mut fight)) = bout(Birdie, Revision::Sf5Ce, Loadout::Skill1Trigger1) else {
        return;
    };
    let assets = session.assets();
    fill_with_break_time(&mut fight, assets);
    let mut unguarded = fight.clone();
    step(&mut unguarded, assets, input(RIGHT, &[LP, MP, HP]));
    assert_ne!(move_id(&unguarded), 0x2d, "V-Reversal requires blockstun");
    assert_eq!(v_gauge(&unguarded), v_gauge(&fight));
    for _ in 0..48 {
        fight.step(assets, pad(RIGHT, None), pad(LEFT, None), false);
    }
    wait(&mut fight, assets, 5);
    let mut blocked = false;
    for tick in 0..22 {
        let p2 = if tick == 0 {
            input(0, &[HP])
        } else {
            PlayerInputs::default()
        };
        fight.step(assets, pad(LEFT, None), p2, false);
        if fight.fight_read().fighters[0].blockstun {
            blocked = true;
            break;
        }
    }
    assert!(blocked, "Ken's attack enters real blockstun");
    let before = fight.machine().era_fighters()[1].sub_hp;
    step(&mut fight, assets, input(RIGHT, &[LP, MP, HP]));
    assert_eq!(move_id(&fight), 0x2d);
    assert_eq!(
        v_gauge(&fight),
        GaugeRow::stock(2, 1, Fill::of(0, STOCK.into()))
    );
    wait(&mut fight, assets, 30);
    assert!(fight.machine().era_fighters()[1].sub_hp < before);
    wait(&mut fight, assets, 115);
    assert_eq!(fight.machine().era_fighters()[1].sub_hp, before);
}

#[test]
fn a_designated_heavy_counter_hit_announces_crush_counter() {
    let Some((session, mut base)) = bout(Ryu, Revision::Sf5Ce, Loadout::Skill1Trigger1) else {
        return;
    };
    let assets = session.assets();
    for _ in 0..50 {
        base.step(assets, pad(RIGHT, None), pad(LEFT, None), false);
    }
    wait(&mut base, assets, 5);
    for timing in 1..9 {
        let mut fight = base.clone();
        for tick in 0..48 {
            let p1 = if tick == 0 {
                input(0, &[HK])
            } else {
                PlayerInputs::default()
            };
            let p2 = if tick == timing {
                input(0, &[HP])
            } else {
                PlayerInputs::default()
            };
            let frame = fight.step(assets, p1, p2, false);
            if frame.announcements.contains(&AnnouncerLine::CrushCounter) {
                assert!(fight.machine().era_fighters()[1].sub_hp < 960);
                assert!(matches!(
                    fight.fight_read().fighters[1].move_id,
                    0x35..=0x3f
                ));
                return;
            }
        }
    }
    panic!("Ryu's heavy kick did not Crush Counter the attack startup");
}

#[test]
fn v_trigger_and_v_gauge_reset_while_ex_carries_into_the_next_round() {
    use sf_engine::fighter::{P2_STRUCT, WORK_BASE};
    use sf_engine::tasks::TIME_DIGITS;
    let Some((session, mut fight)) = bout(Birdie, Revision::Sf5Ce, Loadout::Skill1Trigger1) else {
        return;
    };
    let assets = session.assets();
    fill_with_break_time(&mut fight, assets);
    set_ex(&mut fight, 2 * STOCK);
    step(&mut fight, assets, input(0, &[HP, HK]));
    wait(&mut fight, assets, 30);
    let mut snapshot = fight.save_netplay_snapshot(0);
    let health = (P2_STRUCT + 6 - WORK_BASE) as usize;
    snapshot.machine.work[health] = 1;
    snapshot.machine.work[health + 1] = 0;
    snapshot.machine.toggle_state.era.fighters[1].sub_hp = 20;
    let timer = (TIME_DIGITS - WORK_BASE) as usize;
    snapshot.machine.work[timer..timer + 3].fill(0);
    fight.restore_netplay_snapshot(&snapshot);
    for _ in 0..900 {
        step(&mut fight, assets, PlayerInputs::default());
        if fight.machine().work_ram()[0x400a] == 1
            && move_id(&fight) == 1
            && fight.fight_read().fighters[1].health > 1
        {
            assert_eq!(fight.machine().era_fighters()[0].super_meter, 2 * STOCK);
            assert_eq!(
                v_gauge(&fight),
                GaugeRow::stock(2, 0, Fill::of(0, STOCK.into()))
            );
            return;
        }
    }
    panic!("round two did not reach idle");
}

#[test]
fn champion_edition_ken_uses_ryusenkyaku_and_a_kick_critical_art() {
    use TournamentFighter::Ken;
    let Some((session, mut fight)) = bout(Ken, Revision::Sf5Ce, Loadout::Skill2Trigger2) else {
        return;
    };
    let assets = session.assets();
    assert_eq!(
        v_gauge(&fight),
        GaugeRow::stock(3, 0, Fill::of(0, STOCK.into()))
    );
    step(&mut fight, assets, input(0, &[MP, MK]));
    assert_eq!(move_id(&fight), 0x2c, "Ryusenkyaku");
    wait(&mut fight, assets, 45);
    set_ex(&mut fight, 3 * STOCK);
    let frame = command(&mut fight, assets, QCF_X2, &[HK]);
    assert_eq!(move_id(&fight), 0x29, "Guren Enjinkyaku");
    assert!(frame.announcements.contains(&AnnouncerLine::Super));
    assert_eq!(fight.machine().era_fighters()[0].super_meter, 0);
}

#[test]
fn a_what_if_fighter_uses_the_same_native_v_system() {
    use TournamentFighter::Retsu;
    let Some((session, mut fight)) = bout(Retsu, Revision::Sf5Ce, Loadout::Skill2Trigger2) else {
        return;
    };
    let assets = session.assets();
    assert_eq!(
        v_gauge(&fight),
        GaugeRow::stock(3, 0, Fill::of(0, STOCK.into()))
    );
    step(&mut fight, assets, input(0, &[MP, MK]));
    assert_eq!(move_id(&fight), 0x2c, "Rising Dragon Knee");
    wait(&mut fight, assets, 45);
}
