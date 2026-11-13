//! Exercise the signature effects through the native Training fight.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod support;

use sf_engine::data_bank::{ALPHA_SUPER_SECOND_SLOT, SF2_SUPER_SLOT};
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::{fight::Fight, PlayerInputs};
use sf_engine::moves::{ButtonClass, Dir, MoveInput, Strength};
use sf_engine::roster::TournamentFighter::{Gen, Ken, Ryu, Sagat};
use sf_video::render::Frame;
use sf_video::romset::Assets;
use support::{local_fight, local_select_with_p1, pad, training_fight_with_fighter, DOWN, LEFT, RIGHT, UP};

const HP: usize = 2;
const DENJIN_CHARGE: u8 = 0x20;
const HADOKEN: u8 = 0x1a;
const HASHOGEKI: u8 = 0x1f;
const QCF: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT];
const QCB: &[u8] = &[DOWN, DOWN | LEFT, LEFT];
const QCF_TWICE: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];

struct Comparison<'a> {
    assets: &'a Assets,
    current: Fight,
    preview: Fight,
}

impl<'a> Comparison<'a> {
    fn new(fight: &Fight, assets: &'a Assets) -> Self {
        assert_eq!(fight.move_effects_preview(), Some(false));
        let mut preview = fight.clone();
        preview.toggle_move_effects();
        assert_eq!(preview.move_effects_preview(), Some(true));
        Self {
            assets,
            current: fight.clone(),
            preview,
        }
    }

    fn step(&mut self, p1: PlayerInputs) -> bool {
        let [current, preview] = self.step_frames(p1, PlayerInputs::default());
        current.argb != preview.argb
    }

    fn step_frames(&mut self, p1: PlayerInputs, p2: PlayerInputs) -> [Frame; 2] {
        let current = self.current.step(self.assets, p1, p2, false);
        let preview = self.preview.step(self.assets, p1, p2, false);
        assert_eq!(current.impacts, preview.impacts);
        assert_eq!(current.sound_latch, preview.sound_latch);
        assert_eq!(self.current.fight_read(), self.preview.fight_read());
        assert_eq!(self.current.state_checksum(), self.preview.state_checksum());
        [current.frame, preview.frame]
    }

    fn command(&mut self, motion: &[u8]) -> bool {
        self.command_with_buttons(motion, &[HP])
    }

    fn command_with_buttons(&mut self, motion: &[u8], buttons: &[usize]) -> bool {
        let (&last, first) = motion.split_last().expect("non-empty motion");
        for &direction in first {
            self.step(pad(direction, None));
        }
        let mut input = pad(last, None);
        for &button in buttons {
            input.buttons[button] = true;
        }
        self.step(input)
    }

    fn move_id(&self) -> u8 {
        self.current.fight_read().fighters[0].move_id
    }

    fn hit(&self) -> bool {
        self.current.fight_read().landed[0].is_some_and(|hit| !hit.chip)
    }

    fn projectile(&self, move_id: u8) -> bool {
        self.current
            .machine()
            .toggle_state()
            .era
            .projectiles()
            .iter()
            .any(|shot| shot.move_id == move_id && shot.hits_remaining != 0)
    }

    fn approach(&mut self) {
        for _ in 0..120 {
            let pos = self.current.machine().diagnostic_telemetry();
            if pos.p2.center_x - pos.p1.center_x <= 42 {
                for _ in 0..8 {
                    assert!(!self.step(PlayerInputs::default()));
                }
                return;
            }
            assert!(!self.step(pad(RIGHT, None)));
        }
        panic!("fighters never reached close range");
    }
}

#[test]
fn shinku_metsu_and_later_tiger_cannon_have_flight_and_contact_effects() {
    for (fighter, revision, loadout, buttons, slot) in [
        (Ryu, Revision::Sf6, Loadout::Standard, &[HP][..], 0x29),
        (Ryu, Revision::Usf4, Loadout::Ultra1, &[0, 1, 2][..], 0x2a),
        (Sagat, Revision::Usf4, Loadout::Ultra2, &[0, 1, 2][..], 0x2b),
    ] {
        let Some(session) = training_fight_with_fighter(fighter, revision, loadout) else {
            return;
        };
        let mut fight = Comparison::new(session.fight().unwrap(), session.assets());
        fight.command_with_buttons(QCF_TWICE, buttons);
        assert_eq!(fight.move_id(), slot, "{fighter:?} {revision:?} starts");

        let mut contact = false;
        let mut flight_effect = false;
        let mut impact_effect = false;
        let mut settled = false;
        for _ in 0..140 {
            let different = fight.step(PlayerInputs::default());
            contact |= fight.hit();
            flight_effect |= different && !contact && fight.projectile(slot);
            impact_effect |= different && contact && !fight.projectile(slot);
            settled = !different;
        }
        assert!(flight_effect && impact_effect && contact && settled, "{fighter:?} {revision:?} {slot:#x}: flight {flight_effect}, impact {impact_effect}, contact {contact}, settled {settled}");
    }
}

#[test]
fn drive_impact_distinguishes_whiff_hit_and_guard() {
    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard) else {
        return;
    };
    let session = local_fight(&select, Revision::Sf6, Loadout::Standard);
    let idle = PlayerInputs::default();
    let mut impact = idle;
    impact.buttons[HP] = true;
    impact.buttons[5] = true;
    // The swing and the two contact cues each have a distinct palette pen.
    let cue_colors = [0xfff85b47, 0xffffe2af, 0xfffff1c2, 0xff8adce6];
    for (near, guard) in [(false, false), (true, false), (true, true)] {
        let mut fight = Comparison::new(session.fight().unwrap(), session.assets());
        if near {
            fight.approach();
        }
        let mut startup = false;
        let mut gap = false;
        let mut swing = false;
        let mut contacts = 0;
        let mut settled = false;
        for tick in 0..65 {
            let defender = if guard { pad(RIGHT, None) } else { idle };
            let [current, preview] = fight.step_frames(if tick == 0 { impact } else { idle }, defender);
            if tick == 0 {
                assert_eq!(fight.move_id(), 0x2f);
            }
            let [tell_or_swing, swinging, hit_spark, guard_arc] = cue_colors.map(|color| {
                current.argb.iter().zip(&preview.argb).any(|(&old, &new)| old != new && new == color)
            });
            startup |= tell_or_swing && !swinging;
            gap |= startup && !swing && !tell_or_swing;
            swing |= swinging;
            let hit = fight.hit();
            let blocked = fight.current.fight_read().blocked[0];
            if hit || blocked {
                contacts += 1;
                assert_eq!((hit, blocked), (!guard, guard));
                assert_eq!((hit_spark, guard_arc), (hit, blocked));
                let strike = fight.current.machine().diagnostic_telemetry().p1;
                assert_eq!(strike.move_id, 0x2f);
                let offset = usize::from(strike.entity_offset) + 0x28;
                let objectram = &fight.current.machine().state.objectram;
                let handler = u32::from_be_bytes(objectram[offset..offset + 4].try_into().unwrap());
                assert!(sf_engine::data_bank::in_bank(handler), "contact still holds the active pose");
            } else if contacts == 0 {
                assert!(!hit_spark && !guard_arc, "only confirmed contact produces the burst");
            }
            settled = current.argb == preview.argb;
        }
        assert_eq!(contacts, usize::from(near));
        assert!(startup && gap && (near || swing) && settled, "near {near} guard {guard}: startup {startup}, gap {gap}, swing {swing}, settled {settled}");
    }
}

#[test]
fn body_landings_use_slam_dust_from_the_attacking_style() {
    use sf_engine::fighter::{P2_STRUCT, WORK_BASE};
    use sf_engine::impact::{ImpactFighter, ImpactKind};

    let Some(select) = local_select_with_p1(GaugeMode::EraDefault, Revision::Sf6, Loadout::Standard) else {
        return;
    };
    // A native-1987 victim still gets the SF6 attacker's floor effect.
    let session = local_fight(&select, Revision::Arcade1987, Loadout::Standard);
    let idle = PlayerInputs::default();
    let mut impact = idle;
    impact.buttons[HP] = true;
    impact.buttons[5] = true;
    let dust = |current: &[u32], preview: &[u32]| {
        current.iter().zip(preview).any(|(&old, &new)| {
            old != new && matches!(new, 0xffefcf99 | 0xffcba77a | 0xff9e7957 | 0xff634840)
        })
    };
    for (command, is_throw, is_jump, lethal) in [
        (pad(RIGHT, Some(HP)), true, false, false),
        (impact, false, false, true),
        (impact, false, false, false),
        (pad(UP, None), false, true, false),
    ] {
        let mut fight = Comparison::new(session.fight().unwrap(), session.assets());
        if !is_jump {
            fight.approach();
        }
        if lethal {
            let mut snapshot = fight.current.save_netplay_snapshot(0);
            let health = (P2_STRUCT + 6 - WORK_BASE) as usize;
            snapshot.machine.work[health..health + 2].copy_from_slice(&[1, 0]);
            snapshot.machine.toggle_state.era.fighters[1].sub_hp = 4;
            fight.current.restore_netplay_snapshot(&snapshot);
            fight.preview.restore_netplay_snapshot(&snapshot);
        }
        let mut hit_tick = None;
        let mut mat_tick = None;
        let mut floor_tick = None;
        let mut dust_starts = 0;
        let mut dusty = false;
        for tick in 0..110 {
            let [current, preview] = fight.step_frames(if tick == 0 { command } else { idle }, idle);
            if let Some((cue, _)) = fight.current.impact_visuals().find(|(cue, age)| {
                cue.kind == ImpactKind::MatSlam && cue.fighter == ImpactFighter::P2 && *age == 0
            }) {
                use sf_video::render::{VISAREA_X, VISAREA_Y, WIDTH};

                assert!(mat_tick.replace(tick).is_none(), "one shared mat cue per contact");
                let machine = fight.current.machine();
                assert!(!machine.state.scene().flip_screen);
                let x = usize::from(cue.x - machine.camera_x()) - VISAREA_X;
                let y = usize::from(cue.y) - VISAREA_Y;
                let particle = (y + 1) * WIDTH + x + 13;
                assert_eq!(current.argb[particle], 0xffe5c092, "the shared dust is visible");
                assert_ne!(preview.argb[particle], 0xffe5c092, "the preview replaces the shared dust");

                fight.preview.set_screen_shake(false);
                let steady = fight.preview.render(fight.assets);
                fight.preview.set_screen_shake(true);
                let still_world = &steady.argb[120 * WIDTH..170 * WIDTH];
                assert!(&preview.argb[120 * WIDTH..170 * WIDTH] != still_world);
                let agreeing = preview.argb[128 * WIDTH..178 * WIDTH].iter().zip(still_world)
                    .filter(|(moving, still)| moving == still).count();
                assert!(agreeing * 100 >= still_world.len() * 99,
                    "only {agreeing}/{} world pixels kept the 8px jolt", still_world.len());
            }
            if tick == 0 && is_throw {
                assert_eq!(fight.move_id(), sf_engine::combo::throw::THROW_MOVE);
            }
            if fight.hit() {
                hit_tick.get_or_insert(tick);
            }
            let on_floor = dust(&current.argb, &preview.argb);
            if on_floor && !dusty {
                dust_starts += 1;
                floor_tick.get_or_insert(tick);
                let floor_row = sf_engine::data_bank::GROUND as usize - sf_video::render::VISAREA_Y - 1;
                let width = sf_video::render::WIDTH;
                assert!(dust(
                    &current.argb[(floor_row - 13) * width..(floor_row + 1) * width],
                    &preview.argb[(floor_row - 13) * width..(floor_row + 1) * width],
                ));
            }
            dusty = on_floor;
        }
        if is_jump {
            assert_eq!((hit_tick, floor_tick), (None, None));
        } else if is_throw || lethal {
            let hit_tick = hit_tick.expect("confirmed hit or throw damage");
            let floor_tick = floor_tick.expect("native body floor contact draws slam dust");
            assert_eq!(Some(floor_tick), mat_tick, "preview dust starts on the shared mat cue");
            assert_eq!(dust_starts, 1);
            assert_eq!(floor_tick == hit_tick, is_throw, "throw damage occurs at the slam, strikes land before the victim does");
        } else {
            assert!(hit_tick.is_some(), "the live Drive Impact hits");
            assert!(!fight.current.fight_read().fighters[1].knocked_out());
            assert_eq!((floor_tick, mat_tick), (None, None), "feet-first recovery has no slam dust");
        }
        assert!(!dusty);
    }

    let original_attacker = local_select_with_p1(GaugeMode::EraDefault, Revision::Arcade1987, Loadout::Standard).unwrap();
    let swapped = local_fight(&original_attacker, Revision::Sf6, Loadout::Standard);
    let mut fight = Comparison::new(swapped.fight().unwrap(), swapped.assets());
    fight.approach();
    let mut knocked_down = false;
    for tick in 0..110 {
        let attack = if tick == 0 { pad(DOWN, Some(5)) } else { idle };
        let [current, preview] = fight.step_frames(attack, idle);
        let defender = fight.current.fight_read().fighters[1];
        knocked_down |= (0x33..=0x39).contains(&defender.move_id);
        assert!(!dust(&current.argb, &preview.argb), "the 1987 attack keeps its stock floor effect");
    }
    assert!(knocked_down, "the 1987 sweep caused a native knockdown");
}

#[test]
fn drive_rush_has_a_wake_through_the_dash() {
    let Some(session) = training_fight_with_fighter(Ryu, Revision::Sf6, Loadout::Standard) else {
        return;
    };
    let mut fight = Comparison::new(session.fight().unwrap(), session.assets());
    let input = MoveInput {
        motion: &[Dir::Forward, Dir::Neutral, Dir::Forward],
        button: ButtonClass::MediumPair,
    };
    let mut wake = false;
    for pad in input
        .frames(true, Strength::Medium)
        .into_iter()
        .chain(std::iter::repeat_n(PlayerInputs::default(), 26))
    {
        let different = fight.step(pad);
        wake |= different && fight.move_id() == 0x2c;
    }
    assert!(wake);
    assert!(!fight.step(PlayerInputs::default()));
}

#[test]
fn focus_attack_has_ink_during_charge_and_release() {
    let Some(session) = training_fight_with_fighter(Ryu, Revision::Usf4, Loadout::Ultra1) else {
        return;
    };
    let mut fight = Comparison::new(session.fight().unwrap(), session.assets());
    let mut held = PlayerInputs::default();
    held.buttons[1] = true;
    held.buttons[4] = true;
    let mut charging = false;
    for _ in 0..30 {
        let different = fight.step(held);
        charging |= different && fight.move_id() == 0x2e;
    }
    let mut release = false;
    let mut settled = false;
    for _ in 0..50 {
        let different = fight.step(PlayerInputs::default());
        release |= different && fight.move_id() == 0x2f;
        settled = !different;
    }
    assert!(
        charging && release && settled,
        "charge {charging}, release {release}, settled {settled}"
    );
}

#[test]
fn shinryu_reppa_uses_ken_fire_on_the_final_rise() {
    let Some(session) = training_fight_with_fighter(Ken, Revision::Sf6, Loadout::Standard) else {
        return;
    };
    let mut fight = Comparison::new(session.fight().unwrap(), session.assets());
    fight.approach();
    let mut fire = fight.command(QCF_TWICE);
    assert_eq!(fight.move_id(), 0x2b);
    let mut contact = false;
    let mut settled = false;
    for _ in 0..95 {
        let different = fight.step(PlayerInputs::default());
        fire |= different && fight.move_id() == 0x2b;
        contact |= fight.hit();
        settled = !different;
    }
    assert!(
        fire && contact && settled,
        "fire {fire}, contact {contact}, settled {settled}"
    );
}

#[test]
fn denjin_charge_changes_the_next_hadoken_or_hashogeki() {
    let Some(session) = training_fight_with_fighter(Ryu, Revision::Sf6, Loadout::Standard) else {
        return;
    };

    for (motion, slot) in [(QCF, HADOKEN), (QCB, HASHOGEKI)] {
        let mut fight = Comparison::new(session.fight().unwrap(), session.assets());
        if slot == HASHOGEKI {
            fight.approach();
        }

        let mut charge_effect = fight.command(&[DOWN, 0, DOWN]);
        assert_eq!(fight.move_id(), DENJIN_CHARGE);
        for _ in 0..60 {
            let different = fight.step(PlayerInputs::default());
            charge_effect |= different && fight.move_id() == DENJIN_CHARGE;
        }
        let mut casts = Vec::new();
        for charged in [true, false] {
            if slot == HASHOGEKI && !charged {
                fight.approach();
            }
            let mut launched = false;
            let mut release_effect = false;
            let mut flight_effect = false;
            let mut hit = false;
            let mut last_different = fight.command(motion);
            for _ in 0..110 {
                last_different = fight.step(PlayerInputs::default());
                launched |= fight.move_id() == slot;
                hit |= fight.hit();
                // A plain cast can also draw the shared impact and floor cues.
                release_effect |= !hit && last_different && fight.move_id() == slot;
                flight_effect |= !hit && last_different && fight.projectile(slot);
            }
            assert!(launched, "the cast entered move {slot:#04x}");
            assert!(
                hit,
                "move {slot:#04x}, charged {charged}, reached the dummy"
            );
            casts.push((charged, release_effect, flight_effect));
            assert!(!last_different, "the cast effect expires");
        }
        assert!(charge_effect, "the charge produces its own hand effect");
        for (charged, release_effect, flight_effect) in casts {
            assert_eq!(release_effect, charged, "cast effect for move {slot:#04x}");
            if slot == HADOKEN {
                assert_eq!(flight_effect, charged, "Denjin marks the Hadoken in flight");
            }
        }
    }
}

#[test]
fn tiger_cannon_changes_in_flight_and_on_impact() {
    let Some(session) = training_fight_with_fighter(Sagat, Revision::Sfz3, Loadout::AIsm) else {
        return;
    };
    let mut fight = Comparison::new(session.fight().unwrap(), session.assets());
    assert!(!fight.command(QCF_TWICE));
    assert_eq!(fight.move_id(), ALPHA_SUPER_SECOND_SLOT);

    let mut flight_effect = false;
    let mut impact_effect = false;
    let mut contact = false;
    let mut settled = false;
    for _ in 0..150 {
        let different = fight.step(PlayerInputs::default());
        contact |= fight.hit();
        flight_effect |= different && !contact && fight.projectile(ALPHA_SUPER_SECOND_SLOT);
        impact_effect |= different && contact && !fight.projectile(ALPHA_SUPER_SECOND_SLOT);
        settled = !different;
    }
    assert!(contact, "Tiger Cannon reached the dummy");
    assert!(flight_effect, "Tiger Cannon's wake appears during flight");
    assert!(
        impact_effect,
        "Tiger Cannon's impact lingers after the projectile"
    );
    assert!(settled, "Tiger Cannon's effect expires");
}

#[test]
fn zanei_changes_the_rush_and_the_contact() {
    let Some(session) = training_fight_with_fighter(Gen, Revision::Sfz3, Loadout::AIsm) else {
        return;
    };
    let mut fight = Comparison::new(session.fight().unwrap(), session.assets());
    fight.approach();
    let mut rush_effect = fight.command(QCF_TWICE);
    assert_eq!(fight.move_id(), SF2_SUPER_SLOT);

    let mut contact = false;
    let mut impact_effect = false;
    let mut settled = false;
    for _ in 0..130 {
        let different = fight.step(PlayerInputs::default());
        let hit = fight.hit();
        contact |= hit;
        rush_effect |= different && !contact;
        impact_effect |= different && hit;
        settled = !different;
    }
    assert!(contact, "Zan'ei reached the dummy");
    assert!(
        rush_effect,
        "Zan'ei changes the presentation before contact"
    );
    assert!(
        impact_effect,
        "Zan'ei changes the presentation on a real hit"
    );
    assert!(settled, "Zan'ei's effect expires");
}
