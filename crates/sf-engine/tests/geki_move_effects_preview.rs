//! Exercise Geki's move effects through native Training in both SF1 styles.

#[allow(dead_code)]
#[path = "support/alpha.rs"]
mod support;

use sf_engine::era::{Loadout, Revision};
use sf_engine::fighter::{P1_STRUCT, WORK_BASE};
use sf_engine::front::PlayerInputs;
use sf_engine::roster::TournamentFighter::Geki;
use support::{pad, training_fight_with_fighter, DOWN, LEFT, LP, RIGHT};

#[test]
fn geki_has_shuriken_and_warp_effects_only_in_later_styles() {
    for revision in [Revision::Sf1Plus, Revision::Arcade1987] {
        let Some(session) = training_fight_with_fighter(Geki, revision, Loadout::Standard) else {
            return;
        };
        let assets = session.fight_assets().unwrap();
        let expected = revision == Revision::Sf1Plus;
        for (direction, move_id) in [(RIGHT, 0x1a), (LEFT, 0x1b)] {
            let mut current = session.fight().unwrap().clone();
            let mut preview = current.clone();
            assert_eq!(current.move_effects_preview(), Some(false));
            preview.toggle_move_effects();
            assert_eq!(preview.move_effects_preview(), Some(true));

            let dummy = PlayerInputs::default();
            let initial_health = current.fight_read().fighters[1].health;
            let mut saw_move = false;
            let mut saw_smoke = false;
            let mut saw_effect = false;
            let mut saw_contact = false;
            let mut saw_contact_effect = false;
            let inputs = [
                (pad(DOWN, None), 3),
                (pad(DOWN | direction, None), 3),
                (pad(direction, None), 2),
                (pad(direction, Some(LP)), 2),
                (PlayerInputs::default(), 120),
            ];
            for p1 in inputs
                .into_iter()
                .flat_map(|(input, ticks)| std::iter::repeat_n(input, ticks))
            {
                let before = current.step(assets, p1, dummy, false);
                let after = preview.step(assets, p1, dummy, false);
                assert_eq!(current.fight_read(), preview.fight_read());
                assert_eq!(current.state_checksum(), preview.state_checksum());

                let read = current.fight_read();
                let different = before.frame.argb != after.frame.argb;
                if read.fighters[0].move_id == move_id {
                    saw_move = true;
                    let player = (P1_STRUCT + 0x10 - WORK_BASE) as usize;
                    let work = current.machine().work_ram();
                    let entity = usize::from(u16::from_be_bytes([work[player], work[player + 1]]));
                    let smoke = current.machine().state.objectram[entity + 0xf] & 8 != 0;
                    saw_smoke |= direction == LEFT && smoke;
                    if (direction == LEFT && smoke)
                        || (direction == RIGHT && read.fighters[1].health == initial_health)
                    {
                        saw_effect |= different;
                        if !expected {
                            assert!(
                                !different,
                                "1987 Geki has preview pixels on move {move_id:#x}"
                            );
                        }
                    } else if direction == LEFT && saw_smoke {
                        break;
                    }
                }
                if !saw_contact && read.fighters[1].health < initial_health {
                    saw_contact = true;
                    saw_contact_effect = different;
                }
                if direction == RIGHT && saw_contact {
                    break;
                }
            }
            assert!(saw_move, "{revision:?} Geki never used {move_id:#x}");
            assert_eq!(saw_effect, expected, "{revision:?} move {move_id:#x}");
            if direction == RIGHT {
                assert!(saw_contact, "{revision:?} shuriken never hit the dummy");
                assert_eq!(saw_contact_effect, expected, "{revision:?} shuriken impact");
            } else {
                assert!(
                    saw_smoke,
                    "{revision:?} teleport never entered its smoke phase"
                );
            }
        }
    }
}
