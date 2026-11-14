//! Run the preview and current presentation against the same real fight inputs.

#[path = "support/alpha.rs"]
mod support;

use sf_engine::data_bank::SF2_SUPER_SLOT;
use sf_engine::era::{GaugeMode, Loadout, Revision};
use sf_engine::front::fight::Fight;
use sf_engine::front::{FrameInputs, MenuItem, PlayerInputs, Transition};
use sf_video::compositor::PaletteTint;
use sf_video::move_effects::signature::{signature_impact, ImpactKind};
use sf_video::romset::Assets;
use support::{local_fight, local_select_with_p1, pad, DOWN, LEFT, LP, RIGHT, UP};

const HP: usize = 2;
const MP: usize = 1;

fn preview_pair(current: Fight) -> [Fight; 2] {
    let mut preview = current.clone();
    assert_eq!(current.move_effects_preview(), Some(false));
    preview.toggle_move_effects();
    assert_eq!(preview.move_effects_preview(), Some(true));
    [current, preview]
}

fn step_pair(fights: &mut [Fight; 2], assets: &Assets, p1: PlayerInputs, p2: PlayerInputs) -> bool {
    let [current, preview] = fights;
    let original = current.step(assets, p1, p2, false);
    let new = preview.step(assets, p1, p2, false);
    assert_eq!(current.fight_read(), preview.fight_read());
    assert_eq!(current.state_checksum(), preview.state_checksum());
    original.frame.argb != new.frame.argb
}

fn spark_pixel_count(fights: &[Fight; 2], assets: &Assets, kind: ImpactKind) -> usize {
    let colors: Vec<_> = signature_impact(kind, 0, 0, 0, false)
        .into_iter()
        .flat_map(|sprite| sprite.palette)
        .filter(|&color| color != 0)
        .collect();
    let [current, preview] = fights;
    current
        .render(assets)
        .argb
        .iter()
        .zip(&preview.render(assets).argb)
        .filter(|(old, new)| old != new && colors.contains(new))
        .count()
}

fn approach(fight: &mut Fight, assets: &Assets) {
    let idle = PlayerInputs::default();
    for _ in 0..100 {
        let telemetry = fight.machine().diagnostic_telemetry();
        if telemetry.p2.center_x - telemetry.p1.center_x <= 38 {
            for _ in 0..8 {
                fight.step(assets, idle, idle, false);
            }
            return;
        }
        fight.step(assets, pad(RIGHT, None), idle, false);
    }
    panic!("fighters never reached close range");
}

#[test]
fn the_local_pause_menu_switches_the_comparison() {
    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf1Plus, Loadout::Standard)
    else {
        return;
    };
    let mut session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let mut start = FrameInputs::default();
    start.menu.start = true;
    assert_eq!(session.step(start).transition, Some(Transition::Paused));
    session.step(FrameInputs::default());

    let up = FrameInputs {
        p1: pad(UP, None),
        ..FrameInputs::default()
    };
    let right = FrameInputs {
        p1: pad(RIGHT, None),
        ..FrameInputs::default()
    };
    session.step(up);
    session.step(FrameInputs::default());
    assert_eq!(
        session.step(right).transition,
        Some(Transition::Choose(MenuItem::MoveEffects))
    );
    assert_eq!(session.fight().unwrap().move_effects_preview(), Some(true));
    session.step(FrameInputs::default());
    assert_eq!(
        session.step(right).transition,
        Some(Transition::Choose(MenuItem::MoveEffects))
    );
    assert_eq!(session.fight().unwrap().move_effects_preview(), Some(false));
}

#[test]
fn landing_dust_starts_when_the_jump_lands_and_clears() {
    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf1Plus, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut fights = preview_pair(session.fight().unwrap().clone());
    let idle = PlayerInputs::default();
    let ground = fights[0].machine().diagnostic_telemetry().p1.pushbox.bottom;
    let mut was_airborne = false;
    let mut saw_dust = false;
    let mut settled = false;

    for tick in 0..80 {
        let p1 = if tick == 0 { pad(UP, None) } else { idle };
        let different = step_pair(&mut fights, assets, p1, idle);
        let fighter_bottom = fights[0].machine().diagnostic_telemetry().p1.pushbox.bottom;
        if fighter_bottom < ground - 12 {
            was_airborne = true;
            assert!(!different, "dust appeared in the air");
        } else if different {
            assert!(was_airborne, "dust appeared before a jump landed");
            saw_dust = true;
        } else if saw_dust {
            settled = true;
            break;
        }
    }
    assert!(
        was_airborne && saw_dust && settled,
        "landing dust plays through and clears"
    );
}

#[test]
fn light_medium_and_heavy_sparks_replace_stock_after_contact() {
    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf1Plus, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    approach(&mut base, assets);
    let idle = PlayerInputs::default();

    let mut pixel_counts = Vec::new();
    for (button, kind) in [
        (LP, ImpactKind::LightHit),
        (MP, ImpactKind::MediumHit),
        (HP, ImpactKind::HeavyHit),
    ] {
        let mut fights = preview_pair(base.clone());
        let mut contact = false;
        for tick in 0..24 {
            let p1 = if tick == 0 {
                pad(0, Some(button))
            } else {
                idle
            };
            let different = step_pair(&mut fights, assets, p1, idle);
            contact |= fights[0].fight_read().landed[0].is_some();
            assert!(!different || contact, "a hit spark appeared before contact");
            if different {
                let count = spark_pixel_count(&fights, assets, kind);
                assert!(count > 0, "{kind:?} painted its colors");
                pixel_counts.push(count);
                let [current, preview] = &mut fights;
                preview.toggle_move_effects();
                assert_eq!(current.render(assets).argb, preview.render(assets).argb);
                preview.toggle_move_effects();
                assert_ne!(current.render(assets).argb, preview.render(assets).argb);
                break;
            }
        }
        assert!(contact, "the test punch landed");
    }
    assert_eq!(pixel_counts.len(), 3);
    assert!(
        pixel_counts.windows(2).all(|pair| pair[0] < pair[1]),
        "{pixel_counts:?}"
    );
}

#[test]
fn blocked_normals_keep_stock_presentation() {
    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf1Plus, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    approach(&mut base, assets);
    let mut fights = preview_pair(base);
    let idle = PlayerInputs::default();
    let mut blocked = false;
    for tick in 0..35 {
        let attack = if tick == 0 { pad(0, Some(HP)) } else { idle };
        assert!(!step_pair(&mut fights, assets, attack, pad(RIGHT, None)));
        blocked |= fights[0].fight_read().blocked[0];
    }
    assert!(blocked);
}

#[test]
fn mixed_era_hadoken_spark_uses_its_owner_and_preserves_guard() {
    let Some(select) = local_select_with_p1(
        GaugeMode::EraDefault,
        Revision::Arcade1987,
        Loadout::Standard,
    ) else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let idle = PlayerInputs::default();
    for (attacker, guard) in [(0, false), (1, false), (1, true)] {
        let mut fights = preview_pair(session.fight().unwrap().clone());
        let toward = if attacker == 0 { RIGHT } else { LEFT };
        let mut landed = false;
        let mut chip = false;
        let mut changed = false;
        for tick in 0..100 {
            let mut inputs = [idle; 2];
            inputs[attacker] = match tick {
                0 => pad(DOWN, None),
                1 => pad(DOWN | toward, None),
                2 => pad(toward, Some(MP)),
                _ => idle,
            };
            if guard {
                inputs[1 - attacker] = pad(toward, None);
            }
            let differs = step_pair(&mut fights, assets, inputs[0], inputs[1]);
            if let Some(hit) = fights[0].fight_read().landed[attacker] {
                if !hit.chip && attacker == 1 {
                    assert!(spark_pixel_count(&fights, assets, ImpactKind::MediumHit) > 0);
                }
                landed |= !hit.chip;
                chip |= hit.chip;
            }
            chip |= fights[0].fight_read().blocked[attacker];
            assert!(!differs || landed, "a hit spark appeared before contact");
            changed |= differs;
            if landed || chip {
                break;
            }
        }
        assert_eq!((landed, chip), (!guard, guard), "attacker {attacker}");
        assert_eq!(
            changed,
            attacker == 1 && !guard,
            "attacker {attacker}, guarded {guard}"
        );
    }
}

#[test]
fn mixed_era_landing_dust_belongs_to_the_fighter_who_lands() {
    let Some(select) = local_select_with_p1(
        GaugeMode::EraDefault,
        Revision::Arcade1987,
        Loadout::Standard,
    ) else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let base = session.fight().unwrap();
    assert_eq!(
        base.machine().era_fighters()[0].style.revision,
        Revision::Arcade1987
    );

    for jumper in 0..2 {
        let mut fights = preview_pair(base.clone());
        let feet = |fight: &Fight| {
            let telemetry = fight.machine().diagnostic_telemetry();
            [telemetry.p1, telemetry.p2][jumper].pushbox.bottom
        };
        let ground = feet(&fights[0]);
        let mut airborne = false;
        let mut changed = false;
        for tick in 0..80 {
            let mut inputs = [PlayerInputs::default(); 2];
            if tick == 0 {
                inputs[jumper] = pad(UP, None);
            }
            changed |= step_pair(&mut fights, assets, inputs[0], inputs[1]);
            airborne |= feet(&fights[0]) < ground - 12;
        }
        assert!(
            airborne && feet(&fights[0]) >= ground - 12,
            "side {jumper} jumped and landed"
        );
        assert_eq!(
            changed,
            jumper == 1,
            "only the SF1+ fighter shows preview dust"
        );
    }
}

#[test]
fn mixed_era_heavy_spark_belongs_to_the_attacker() {
    let Some(select) = local_select_with_p1(
        GaugeMode::EraDefault,
        Revision::Arcade1987,
        Loadout::Standard,
    ) else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    assert_eq!(
        base.machine().era_fighters()[0].style.revision,
        Revision::Arcade1987
    );
    approach(&mut base, assets);

    for attacker in 0..2 {
        let mut fights = preview_pair(base.clone());
        let mut contact = false;
        let mut changed = false;
        for tick in 0..24 {
            let mut inputs = [PlayerInputs::default(); 2];
            if tick == 0 {
                inputs[attacker] = pad(0, Some(HP));
            }
            let different = step_pair(&mut fights, assets, inputs[0], inputs[1]);
            contact |= fights[0].fight_read().landed[attacker].is_some();
            assert!(!different || contact, "a hit spark appeared before contact");
            changed |= different;
        }
        assert!(contact, "side {attacker}'s heavy punch landed");
        assert_eq!(
            changed,
            attacker == 1,
            "only the SF1+ attacker swaps the spark"
        );
    }
}

#[test]
fn the_sf3_parry_flash_requires_a_successful_parry() {
    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf3ThreeS, Loadout::Art1)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sf1Plus, Loadout::Standard);
    let assets = session.assets();
    let mut base = session.fight().unwrap().clone();
    approach(&mut base, assets);
    let idle = PlayerInputs::default();
    let jab = pad(0, Some(LP));
    let mut probe = base.clone();
    let impact = (0usize..30)
        .find(|&tick| {
            probe.step(assets, idle, if tick == 0 { jab } else { idle }, false);
            probe.fight_read().landed[1].is_some()
        })
        .expect("the incoming jab would hit");

    let mut fights = preview_pair(base);
    let mut parried = false;
    let mut saw_flash = false;
    for tick in 0..=impact + 8 {
        let p1 = if tick == impact.saturating_sub(1) {
            pad(RIGHT, None)
        } else {
            idle
        };
        let p2 = if tick == 0 { jab } else { idle };
        let different = step_pair(&mut fights, assets, p1, p2);
        parried |= fights[0].machine().era_super_freeze() > 0;
        assert!(
            !different || parried,
            "a tap alone did not draw the parry flash"
        );
        saw_flash |= different;
        assert!(fights[0].fight_read().landed[1].is_none());
    }
    assert!(
        parried && saw_flash,
        "the successful parry draws the preview flash"
    );
}

#[test]
fn alpha_ken_heavy_shoryuken_can_burn_an_sf1_opponent() {
    let Some(select) = local_select_with_p1(
        GaugeMode::EraDefault,
        Revision::Arcade1987,
        Loadout::Standard,
    ) else {
        return;
    };
    let session = local_fight(&select, Revision::Sfz2, Loadout::Standard);
    let assets = session.assets();
    let idle = PlayerInputs::default();
    let base = session.fight().unwrap();
    assert_eq!(
        base.machine().era_fighters()[0].style.revision,
        Revision::Arcade1987
    );

    for button in [LP, HP] {
        let mut fights = preview_pair(base.clone());
        for direction in [LEFT, DOWN] {
            assert!(!step_pair(&mut fights, assets, idle, pad(direction, None)));
        }
        let mut saw_fire = step_pair(&mut fights, assets, idle, pad(DOWN | LEFT, Some(button)));
        assert_eq!(
            fights[0].fight_read().fighters[1].move_id,
            0x19,
            "Ken performed Shoryuken"
        );
        for _ in 0..12 {
            saw_fire |= step_pair(&mut fights, assets, idle, idle);
            assert!(
                fights[0].fight_read().landed[1].is_none(),
                "the Shoryuken whiffs at starting distance"
            );
        }
        assert_eq!(
            saw_fire,
            button == HP,
            "the flame marks Ken's heavy Shoryuken"
        );
    }

    let mut close = base.clone();
    approach(&mut close, assets);
    let mut fights = preview_pair(close);
    for direction in [LEFT, DOWN] {
        step_pair(&mut fights, assets, idle, pad(direction, None));
    }
    for tick in 0..15 {
        let p2 = if tick == 0 {
            pad(DOWN | LEFT, Some(HP))
        } else {
            idle
        };
        step_pair(&mut fights, assets, idle, p2);
        if fights[0].fight_read().landed[1].is_some() {
            let [current, preview] = fights;
            let changed_pixels = current
                .render(assets)
                .argb
                .into_iter()
                .zip(preview.render(assets).argb)
                .filter(|(current, preview)| current != preview)
                .count();
            assert!(
                changed_pixels > 16 * 16,
                "the SF1 opponent burns at contact, beyond the single fist-flame tile"
            );
            return;
        }
    }
    panic!("the close heavy Shoryuken did not hit");
}

#[test]
fn alpha_ken_level_three_shoryureppa_burns_on_the_final_rise() {
    let Some(select) =
        local_select_with_p1(GaugeMode::EraDefault, Revision::Sf1Plus, Loadout::Standard)
    else {
        return;
    };
    let session = local_fight(&select, Revision::Sfz3, Loadout::AIsm);
    let assets = session.assets();
    let idle = PlayerInputs::default();
    let mut base = session.fight().unwrap().clone();
    approach(&mut base, assets);
    let mut snapshot = base.save_netplay_snapshot(0);
    snapshot.machine.toggle_state.era.fighters[1].super_meter = 144;
    base.restore_netplay_snapshot(&snapshot);

    for (button, expected) in [
        (LP, &[false, false][..]),
        (HP, &[false, false, true, true][..]),
    ] {
        let mut fights = preview_pair(base.clone());
        for direction in [DOWN, DOWN | LEFT, LEFT, DOWN] {
            assert!(!step_pair(&mut fights, assets, idle, pad(direction, None)));
        }
        assert!(!step_pair(
            &mut fights,
            assets,
            idle,
            pad(DOWN | LEFT, Some(button))
        ));
        assert_eq!(fights[0].fight_read().fighters[1].move_id, SF2_SUPER_SLOT);
        let mut burns_on_hit = Vec::new();
        for _ in 0..150 {
            step_pair(&mut fights, assets, idle, idle);
            if fights[0].fight_read().landed[1].is_some() {
                let [current, preview] = &fights;
                let tint = PaletteTint {
                    color: 0xffffdb72,
                    strength: 180,
                };
                let tinted_pixels = current
                    .render(assets)
                    .argb
                    .into_iter()
                    .zip(preview.render(assets).argb)
                    .filter(|&(old, new)| old != new && tint.apply(old) == new)
                    .count();
                burns_on_hit.push(tinted_pixels > 16);
            }
        }
        assert_eq!(
            burns_on_hit, expected,
            "only the level-three finisher burns"
        );
    }
}
