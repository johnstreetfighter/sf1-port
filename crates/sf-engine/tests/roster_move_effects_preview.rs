#[path = "support/alpha.rs"]
#[allow(dead_code)]
mod alpha;

use alpha::{pad, training_fight_with_fighter, DOWN, LEFT, RIGHT};
use sf_engine::era::{Loadout, Revision};
use sf_engine::front::PlayerInputs;
use sf_engine::roster::TournamentFighter::{self, Birdie, Eagle, Joe, Lee, Mike, Retsu};
use sf_video::move_effects::accent::{accent_frame, AccentKind, AccentKind::*};

const HP: usize = 2;
const HK: usize = 5;
const QCF: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT];
const QCB: &[u8] = &[DOWN, DOWN | LEFT, LEFT];
const DP: &[u8] = &[RIGHT, DOWN, DOWN | RIGHT];
const QCF_TWICE: &[u8] = &[DOWN, DOWN | RIGHT, RIGHT, DOWN, DOWN | RIGHT, RIGHT];
const QCB_TWICE: &[u8] = &[DOWN, DOWN | LEFT, LEFT, DOWN, DOWN | LEFT, LEFT];

fn check_preview(
    fighter: TournamentFighter,
    (revision, loadout): (Revision, Loadout),
    charge: bool,
    motion: &[u8],
    button: usize,
    slot: u8,
    accent: AccentKind,
) {
    let Some(session) = training_fight_with_fighter(fighter, revision, loadout) else {
        return;
    };
    let mut current = session.fight().unwrap().clone();
    let mut preview = current.clone();
    preview.toggle_move_effects();
    assert_eq!(current.move_effects_preview(), Some(false));
    assert_eq!(preview.move_effects_preview(), Some(true));

    let colors: Vec<_> = (0..8)
        .flat_map(|age| accent_frame(accent, age, 0, 0, false))
        .flat_map(|sprite| sprite.palette)
        .filter(|&color| color != 0)
        .collect();
    let mut entered = false;
    let mut painted = false;
    let mut recovered = false;
    let mut inputs = Vec::new();
    if charge {
        inputs.extend([pad(DOWN | LEFT, None); 44]);
    }
    let (&last, directions) = motion.split_last().unwrap();
    inputs.extend(directions.iter().map(|&direction| pad(direction, None)));
    inputs.push(pad(last, Some(button)));
    inputs.extend([PlayerInputs::default(); 120]);
    for input in inputs {
        let neutral = PlayerInputs::default();
        let native = current.step(session.assets(), input, neutral, false);
        let enhanced = preview.step(session.assets(), input, neutral, false);
        assert_eq!(current.state_checksum(), preview.state_checksum());
        let in_move = current.fight_read().fighters[0].move_id == slot;
        let draws_accent = enhanced
            .frame
            .argb
            .iter()
            .zip(&native.frame.argb)
            .any(|(after, before)| after != before && colors.contains(after));
        if entered && !in_move {
            assert!(
                !draws_accent,
                "{accent:?} remained after {fighter:?} recovered"
            );
            recovered = true;
            break;
        }
        entered |= in_move;
        painted |= in_move && draws_accent;
    }
    assert!(entered, "{fighter:?} entered move {slot:#04x}");
    assert!(painted, "{fighter:?} move {slot:#04x} drew {accent:?}");
    assert!(recovered, "{fighter:?} recovered from move {slot:#04x}");
}

macro_rules! preview_cases {
    ($($test:ident: $fighter:ident, $revision:ident, $loadout:ident, $charge:expr,
      $motion:expr, $button:expr, $slot:expr, $accent:ident;)*) => {
        $(
            #[test]
            fn $test() {
                check_preview($fighter, (Revision::$revision, Loadout::$loadout),
                              $charge, $motion, $button, $slot, $accent);
            }
        )*
    };
}

preview_cases! {
    bull_head: Birdie, Sf1Plus, Standard, false, QCF, HP, 0x1a, BirdieRush;
    the_birdie: Birdie, Sfz3, AIsm, true, &[RIGHT, LEFT, RIGHT], HP, 0x29, BirdieSuper;
    manchester_black: Eagle, Sfz3, AIsm, false, QCB, HP, 0x1a, EagleStrike;
    manchester_gold: Eagle, Sfz3, AIsm, false, QCB_TWICE, HP, 0x29, EagleSuper;
    kiai_shot: Retsu, Sf6, Standard, false, QCF, HP, 0x1a, RetsuKiai;
    tobigeri: Retsu, Sf6, Standard, false, QCF, HK, 0x19, RetsuKick;
    knee_bash: Joe, Sf6, Standard, false, DP, HK, 0x1f, JoeKick;
    murder_straight: Mike, Sf6, Standard, false, QCF, HP, 0x1a, MikePunch;
    firewater: Lee, Sf6, Standard, true, &[RIGHT], HP, 0x20, LeePalm;
    super_sobat_rush: Joe, Sf6, Standard, false, QCF_TWICE, HP, 0x29, JoeKick;
    crazy_rush: Mike, Sf6, Standard, false, QCF_TWICE, HP, 0x29, MikePunch;
    hundred_spears: Lee, Sf6, Standard, false, QCF_TWICE, HP, 0x29, LeePalm;
}
