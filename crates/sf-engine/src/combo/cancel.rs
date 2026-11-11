//! Cancels (spec #246, ticket #262, `SP18`): the modern rule that lets a
//! special's command cut a normal short (design
//! `docs/design/2026-11-07-sf1plus-on-the-real-engine-design.md` 5.2,
//! ADR-0014).
//!
//! **The switch.** [`CoreToggles::cancels`](crate::config::CoreToggles) (on
//! in Definitive Overhaul, off in Classic Competitive and Pure Footsies,
//! never on in Faithful 1987). Its gates are on the translated path in
//! [`crate::fighter`]; with it off nothing here runs, the state stays at its
//! default, and the fight is the 1987 fight byte for byte.
//!
//! **What 1987 does.** A fighter in an attack is busy (`+9` bits 7 and 6):
//! the input handler's busy gate (`0x403d4`) sends him to its tail, which
//! drops whatever button was latched (`0x4050c`), so nothing starts until
//! the move's stream ends. The command trackers run every frame before that
//! gate, so a special's motion can be made during a normal; only the button
//! is lost.
//!
//! **What the switch changes.**
//!
//! - **The window opens on contact.** When a normal this table marks
//!   cancelable connects (the collision judge's hit function clears the
//!   attacker's active bit, hit or guarded), the attacker's cancel window
//!   opens for the move's [`window`] frames, counted from the frame after
//!   the contact. A whiff opens nothing (the research's conservative
//!   convention, SF2's: specials cancel a normal on hit or block only).
//!   The window closes early when the move ends, another move starts, or
//!   the fighter is hit.
//! - **A special's command cancels the normal.** In the window, a button
//!   whose special's command is done at the direction held (the same
//!   rule the 1987 dispatch fires specials by,
//!   [`crate::fighter::Bus`]`::special_ready_for_dir`, under strict or
//!   lenient commands) starts the special at once, through the 1987
//!   direction dispatch: the same special, strength and tracker clears as
//!   the same input on a free frame. Any other button does nothing (no
//!   chains into normals). A special pressed in the few frames before the
//!   contact waits in the recovery input buffer and is taken when the window
//!   opens, as modern games buffer a cancel.
//! - **Combos.** A cancel is a new move: the special connects on its own
//!   and, under true combos ([`super`]), counts on into the combo running
//!   (its hit count, damage scaling and the era HUD's counter).
//!
//! **Which normals cancel (the per-move mark).** One rule by the 1987 input
//! handler's move ids, which mean the same posture and button for every
//! fighter on a player table (Ryu's and Ken's ROM tables and the bosses'
//! bank tables alike, `docs/reference/posture-dispatch.md`,
//! `docs/reference/crouch-attacks.md`): the standing light punch and light
//! kick, the standing medium punch, every crouching light punch, the
//! crouching medium punches and the crouching light and medium kicks. Never
//! a heavy, the sweep, a standing medium kick, a jump attack or a special.
//! This is the classic SF2 set (lights, medium punches and the crouching
//! mediums; the research's 3.6: which normals cancel is a balance lever,
//! and ST's Old Sagat lost his standing-kick cancels for spamming
//! projectiles). The 1987 standing medium kick is a slow, long-recovering
//! kick that knocks a standing opponent back through the air, a heavy in
//! all but name, so it stays out.
//!
//! Ryu, Ken, Retsu and Geki all take the rule ([`rows`]); a later boss
//! ticket that wants a different mark for one of his moves gives him his own
//! rows. A fighter without rows (the bosses still on the boss-only controls,
//! which do not run the 1987 handler) also takes the rule.
//!
//! **Determinism.** The window lives in [`CancelState`], which rides in
//! [`crate::config::CoreToggleState`] and so in every machine snapshot.

use crate::roster::TournamentFighter;

/// Frames a cancelable normal's window stays open after its contact (the
/// frames after the contact frame on which the input handler can cancel).
///
/// 1987 has no hitstop, so the window is real time: the research's SF6
/// lights are cancelable through their active frames and some six frames of
/// recovery, plus their 9 to 11 frames of hitstop, about 15 to 18 frames
/// from contact. A shorter window keeps a cancel a reaction to the contact
/// rather than a late afterthought, and a cancel later than the defender's
/// hitstun minus the special's startup cannot combo anyway (Ryu's and Ken's
/// cancelable normals give 9 to 18 frames of hitstun). Every one of their
/// cancelable normals ends within 12 frames of its contact, so for them the
/// window is the rest of the move; Retsu's and Geki's long recoveries are
/// where it closes first.
pub const WINDOW: u8 = 12;

/// The cancel window of each fighter, by [`super::slot`] (one per player
/// structure, as [`super::ComboState`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CancelState {
    /// Frames left of the window (0: closed). Set on the contact, counted
    /// down by the fighter's input handler.
    pub window: [u8; 3],
    /// The move that opened the window: a window is only good while the
    /// fighter is still in it.
    pub move_id: [u8; 3],
    /// Cancels made, for the tests and the log (wraps).
    pub count: [u8; 3],
}

impl crate::canonical::Canonical for CancelState {
    fn encode(&self, out: &mut crate::canonical::CanonicalHasher) {
        let Self { window, move_id, count: _ } = self;
        out.tag("CancelState.v2");
        window.encode(out);
        move_id.encode(out);
        // count is an observation for tests/the log, never a combat input.
    }
}

/// One row: a player-table move id and its window at strength level 1, 2
/// and 3 (0: not cancelable at that level).
type Row = (u8, [u8; 3]);

const W: u8 = WINDOW;

/// The rule every fighter takes today, by the 1987 handler's move ids and
/// strength levels (`crate::combo::level`):
///
/// | Move | Posture, levels | Cancelable |
/// | --- | --- | --- |
/// | 0x0d / 0x0e | standing light punch | yes |
/// | 0x0f | standing medium punch | yes |
/// | 0x10 | standing heavy punch | no |
/// | 0x11 | crouching light punch (down) | yes |
/// | 0x12 / 0x13 | crouching light punch (down-forward / down-back) | yes |
/// | 0x1c / 0x1d / 0x1e | crouching medium (2) and heavy (3) punch | medium only |
/// | 0x14 | standing light kick | yes |
/// | 0x15 | standing medium kick | no |
/// | 0x16 | standing heavy kick | no |
/// | 0x17 | crouching light (1) and medium (2) kick | yes |
/// | 0x18 | sweep | no |
///
/// Moves not listed (the stance, walks, jumps and jump attacks, guards,
/// specials, reactions) never cancel.
const RULE: &[Row] = &[
    (0x0d, [W, W, W]),
    (0x0e, [W, W, W]),
    (0x0f, [W, W, W]),
    (0x11, [W, W, W]),
    (0x12, [W, W, W]),
    (0x13, [W, W, W]),
    (0x14, [W, W, W]),
    (0x17, [W, W, W]),
    (0x1c, [W, W, 0]),
    (0x1d, [W, W, 0]),
    (0x1e, [W, W, 0]),
];

/// The cancel rows of `fighter`'s player moves. Every fighter in the roster
/// takes the rule (see the module docs).
fn rows(_fighter: Option<TournamentFighter>) -> &'static [Row] {
    RULE
}

/// The cancel window, in frames, of move `id` at strength `level` (1..=3)
/// by `fighter`: 0 when the move is not cancelable.
pub fn window(fighter: Option<TournamentFighter>, id: u8, level: u8) -> u8 {
    let level = level.clamp(1, 3);
    rows(fighter)
        .iter()
        .find(|(m, _)| *m == id)
        .map_or(0, |(_, by_level)| by_level[level as usize - 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lights_medium_punches_and_crouching_mediums_cancel_heavies_do_not() {
        for f in TournamentFighter::ALL {
            let f = Some(f);
            for id in [0x0d, 0x0e, 0x11, 0x12, 0x13, 0x14] {
                assert_eq!(window(f, id, 1), WINDOW, "light {id:#04x}");
            }
            assert_eq!(window(f, 0x0f, 2), WINDOW, "standing medium punch");
            assert_eq!(window(f, 0x17, 1), WINDOW, "crouching light kick");
            assert_eq!(window(f, 0x17, 2), WINDOW, "crouching medium kick");
            for id in [0x1c, 0x1d, 0x1e] {
                assert_eq!(window(f, id, 2), WINDOW, "crouching medium punch {id:#04x}");
                assert_eq!(window(f, id, 3), 0, "crouching heavy punch {id:#04x}");
            }
            for id in [0x10, 0x15, 0x16, 0x18] {
                assert_eq!(window(f, id, 3), 0, "{id:#04x}");
                assert_eq!(window(f, id, 2), 0, "{id:#04x}");
            }
            // Stance, walks, guards, jumps, jump attacks, specials, reactions.
            for id in [
                0x01, 0x02, 0x04, 0x08, 0x09, 0x0a, 0x19, 0x1a, 0x1b, 0x20, 0x23, 0x26, 0x30, 0x3a,
            ] {
                assert_eq!(window(f, id, 1), 0, "{id:#04x}");
            }
        }
        assert_eq!(window(None, 0x0d, 1), WINDOW);
    }
}
