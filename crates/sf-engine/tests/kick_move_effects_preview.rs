#[path = "support/alpha.rs"]
mod alpha;

use alpha::{pad, training_fight_with_fighter, DOWN, LEFT, RIGHT};
use sf_engine::era::{Loadout, Revision};
use sf_engine::front::PlayerInputs;
use sf_engine::roster::TournamentFighter::{self, Adon, Ken};
use sf_video::move_effects::accent::{accent_frame, AccentKind};

const LK: usize = 3;
const HK: usize = 5;
const RDP: &[u8] = &[LEFT, DOWN, DOWN | LEFT];
const QCB_TWICE: &[u8] = &[DOWN, DOWN | LEFT, LEFT, DOWN, DOWN | LEFT, LEFT];
const QCF_TWICE: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];

fn compare_kick(
    fighter: TournamentFighter,
    revision: Revision,
    loadout: Loadout,
    motion: &[u8],
    button: usize,
    slot: u8,
    effects: &[AccentKind],
) {
    let Some(session) = training_fight_with_fighter(fighter, revision, loadout) else {
        return;
    };
    let mut current = session.fight().unwrap().clone();
    let mut preview = current.clone();
    preview.toggle_move_effects();
    assert_eq!(preview.move_effects_preview(), Some(true));

    let colors: Vec<_> = effects
        .iter()
        .map(|&kind| accent_frame(kind, 0, 0, 0, false)[0].palette[5])
        .collect();
    let mut active_frames = vec![0; effects.len()];
    let (&last, directions) = motion.split_last().unwrap();
    let inputs = directions
        .iter()
        .map(|&direction| pad(direction, None))
        .chain([pad(last, Some(button))])
        .chain([PlayerInputs::default(); 96]);
    for input in inputs {
        let neutral = PlayerInputs::default();
        let before = current.step(session.assets(), input, neutral, false);
        let after = preview.step(session.assets(), input, neutral, false);
        assert_eq!(current.state_checksum(), preview.state_checksum());
        assert_eq!(current.fight_read(), preview.fight_read());
        for (index, color) in colors.iter().enumerate() {
            let painted = before
                .frame
                .argb
                .iter()
                .zip(&after.frame.argb)
                .any(|(before, after)| before != after && after == color);
            if painted {
                assert_eq!(current.fight_read().fighters[0].move_id, slot);
                active_frames[index] += 1;
            }
        }
    }
    for (kind, count) in effects.iter().zip(active_frames) {
        assert!(
            count >= 2,
            "{fighter:?} {revision:?} {kind:?} painted for {count} frames"
        );
    }
}

#[test]
fn alpha_adon_has_heel_crescents_on_jaguar_kick_and_revolver() {
    compare_kick(
        Adon,
        Revision::Sfz3,
        Loadout::AIsm,
        RDP,
        LK,
        0x1b,
        &[AccentKind::JaguarKick],
    );
    compare_kick(
        Adon,
        Revision::Sfz3,
        Loadout::AIsm,
        QCB_TWICE,
        HK,
        0x2a,
        &[AccentKind::JaguarRevolver],
    );
}

#[test]
fn sf6_ken_has_dragonlash_fire_and_shippu_kicks_with_a_distinct_finisher() {
    compare_kick(
        Ken,
        Revision::Sf6,
        Loadout::Standard,
        QCF_TWICE,
        LK,
        0x29,
        &[AccentKind::DragonlashFlame],
    );
    compare_kick(
        Ken,
        Revision::Sf6,
        Loadout::Standard,
        QCB_TWICE,
        LK,
        0x2a,
        &[AccentKind::ShippuKick, AccentKind::ShippuSpiral],
    );
}

#[test]
fn shinryuken_has_a_flame_column_in_alpha_and_third_strike() {
    for (revision, loadout) in [
        (Revision::Sfz2, Loadout::Standard),
        (Revision::Sf3ThreeS, Loadout::Art2),
    ] {
        compare_kick(
            Ken,
            revision,
            loadout,
            QCF_TWICE,
            LK,
            0x2a,
            &[AccentKind::Shinryuken],
        );
    }
}

#[test]
fn sf5_guren_enjinkyaku_has_a_horizontal_flame_sweep() {
    compare_kick(
        Ken,
        Revision::Sf5Ce,
        Loadout::Skill1Trigger1,
        QCF_TWICE,
        LK,
        0x29,
        &[AccentKind::GurenSweep],
    );
}
