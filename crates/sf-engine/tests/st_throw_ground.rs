//! Super Turbo throws with the shipped art keep both fighters on the mat
//! (George's playtest, 2026-11-17: the thrower was lifted off the ground and
//! the thrown fighter landed on another plane).

#[path = "support/alpha.rs"]
mod support;

use sf_engine::combo::throw::THROWN_MOVE;
use sf_engine::era::{Loadout, Revision};
use sf_engine::front::{FrameInputs, GameSession};
use sf_video::decode::decode_sprite_tile;
use sf_video::sprite_bank::sprite_slot;
use support::{local_fight_p2_ryu, local_select_with_shoto_art, pad, LEFT, RIGHT};

type Tiles = Vec<[[u8; 16]; 16]>;

fn entity(s: &GameSession, side: usize) -> usize {
    let read = s.fight().unwrap().machine().diagnostic_telemetry();
    usize::from([read.p1, read.p2][side].entity_offset)
}

/// The lowest opaque row of the fighter's live sprite entries.
fn opaque_bottom(tiles: &Tiles, s: &GameSession, side: usize) -> i32 {
    let o = &s.fight_machine().unwrap().state.objectram;
    let e = entity(s, side);
    let w = |a| u16::from_be_bytes([o[a], o[a + 1]]);
    let n = ((usize::from(o[e + 0xc]) + 1) * (usize::from(o[e + 0xd]) + 1)).min(24);
    let mut bottom = i32::MIN;
    for k in 0..n {
        let a = e + k * 0x40;
        let (code, attr, y) = (w(a), w(a + 2), i32::from(w(a + 4) as i16));
        let (big, fx, fy) = (attr & 0x400 != 0, attr & 0x100 != 0, attr & 0x200 != 0);
        let mut q = [code, code.wrapping_add(1), code.wrapping_add(16), code.wrapping_add(17)];
        if big && fx {
            q.swap(0, 1);
            q.swap(2, 3);
        }
        if big && fy {
            q.swap(0, 2);
            q.swap(1, 3);
        }
        for (i, &code) in q.iter().enumerate().take(if big { 4 } else { 1 }) {
            let tile = &tiles[sprite_slot(code) % tiles.len()];
            for py in 0..16 {
                if tile[if fy { 15 - py } else { py }].iter().any(|&p| p != 15) {
                    bottom = bottom.max(y + if big { (i / 2) as i32 * 16 } else { 0 } + py as i32);
                }
            }
        }
    }
    bottom
}

fn key(side: usize, directions: u8, button: Option<usize>) -> FrameInputs {
    let mut input = FrameInputs::default();
    if side == 0 {
        input.p1 = pad(directions, button);
    } else {
        input.p2 = pad(directions, button);
    }
    input
}

/// Ryu against Ryu, both sides, every throw button, forward and back, New
/// and Old: the thrower's feet never leave the mat, and the thrown fighter
/// comes to rest on it, lying on his back.
#[test]
fn st_ryu_throws_keep_the_thrower_on_the_mat_and_land_the_thrown_on_it() {
    for loadout in [Loadout::Standard, Loadout::Old] {
        let Some((select, art)) = local_select_with_shoto_art(Revision::St, loadout) else {
            return;
        };
        let mut close = local_fight_p2_ryu(&select, Revision::St, loadout);
        let banked = close.assets().with_sprite_bank(&art.bank);
        let tiles: Tiles = (0..banked.gfx3.len() / 128)
            .map(|n| decode_sprite_tile(&banked.gfx3, n))
            .collect();
        for _ in 0..200 {
            close.step(key(0, RIGHT, None));
        }
        for _ in 0..20 {
            close.step(FrameInputs::default());
        }
        let lying = *art.catalogue("ryu").unwrap().move_poses("thrown").unwrap().last().unwrap();
        let lying_codes = [lying.grid.code, lying.grid.code + 2 * u16::from(lying.grid.cols)];
        assert!(lying.grid.large && lying.name == "lying");
        let mat = opaque_bottom(&tiles, &close, 0);
        assert_eq!(opaque_bottom(&tiles, &close, 1), mat, "both stand on the mat");
        for side in 0..2 {
            let toward = if side == 0 { RIGHT } else { LEFT };
            for button in [1usize, 2, 4, 5] {
                for back in [false, true] {
                    let at = format!("{loadout:?} side {side} button {button} back {back}");
                    let mut s = close.clone();
                    let direction = if back { RIGHT + LEFT - toward } else { toward };
                    let (mut thrown, mut rested) = (false, 0);
                    for tick in 0..220 {
                        s.step(if tick == 0 {
                            key(side, direction, Some(button))
                        } else {
                            FrameInputs::default()
                        });
                        let read = s.fight().unwrap().fight_read();
                        thrown |= read.fighters[1 - side].move_id == THROWN_MOVE;
                        assert_eq!(
                            opaque_bottom(&tiles, &s, side),
                            mat,
                            "{at}: the thrower's feet at tick {tick}"
                        );
                        // The thrown fighter's flight and bounce end by tick
                        // 110; his rest picture's ink ends a pixel above his
                        // standing feet.
                        if tick >= 110 {
                            let ink = opaque_bottom(&tiles, &s, 1 - side);
                            assert!(
                                (mat - 1..=mat).contains(&ink),
                                "{at}: the thrown fighter rests at {ink}, the mat is {mat}, tick {tick}"
                            );
                            let o = &s.fight_machine().unwrap().state.objectram;
                            let e = entity(&s, 1 - side);
                            let code = u16::from_be_bytes([o[e], o[e + 1]]);
                            if read.fighters[1 - side].move_id != 1 {
                                assert!(
                                    lying_codes.contains(&code),
                                    "{at}: the thrown fighter lies on his back at tick {tick}, not {code:#x}"
                                );
                                rested += 1;
                            }
                        }
                    }
                    assert!(thrown && rested > 0, "{at}: the throw connected");
                }
            }
        }
    }
}
