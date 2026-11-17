//! Hit cues painted over the native lifebar's newly empty pixels.

use sf_video::render::{HEIGHT, TextEdits, TextFill, VISAREA_X, VISAREA_Y, WIDTH};

use crate::impact::{ImpactCue, ImpactFighter, ImpactKind};

// The cabinet's two fourteen-cell health bars at 0x8002ba and 0x8003ba.
const TEXT_COLS: usize = 64;
pub(super) const FIRST_COL: usize = 29;
pub(super) const FILL_CELLS: usize = 12;
const FIRST_FILL_X: usize = (FIRST_COL + 1) * 8 - VISAREA_X;
pub(super) const FILL_POINTS: u8 = (FILL_CELLS * 4) as u8;
pub(super) const ROWS: [usize; 2] = [5, 7];

const AMBER: (u8, u8) = (6, 2);
const GOLD: (u8, u8) = (4, 2);
const SHADOW: (u8, u8) = (4, 1);
const LIGHT: (u8, u8) = (4, 0);
const EDGE_GOLD: (u8, u8) = (12, 0);

pub(super) fn native_fill(videoram: &[u8], row: usize) -> Option<u8> {
    let cell = |col| {
        let at = (row * TEXT_COLS + col) * 2;
        u16::from_be_bytes([videoram[at], videoram[at + 1]])
    };
    if cell(FIRST_COL) != 0x1525 || cell(FIRST_COL + FILL_CELLS + 1) != 0x1125 {
        return None;
    }
    let mut points = 0;
    for col in FIRST_COL + 1..FIRST_COL + 1 + FILL_CELLS {
        // draw_health writes 0x1120 for a full cell through 0x1124 for empty.
        let empty = cell(col)
            .checked_sub(0x1120)
            .filter(|&amount| amount <= 4)?;
        points += 4 - empty as u8;
    }
    Some(points)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct LifebarTrails([Option<Trail>; 3]);

impl LifebarTrails {
    pub(super) fn step(
        &mut self,
        before: [Option<u8>; 3],
        after: [Option<u8>; 3],
        heavy: [bool; 3],
        frame: u32,
    ) {
        for (slot, bar) in self.0.iter_mut().enumerate() {
            let (Some(before), Some(fill @ 1..)) = (before[slot], after[slot]) else {
                *bar = None;
                continue;
            };
            if fill < before {
                *bar = Some(Trail {
                    fill,
                    lost: before - fill,
                    frame,
                    heavy: heavy[slot],
                });
            } else if fill > before || bar.is_some_and(|trail| trail.view(fill, frame).is_none()) {
                *bar = None;
            }
        }
    }

    pub(super) fn paint_native(
        &self,
        edits: &mut TextEdits,
        videoram: &[u8],
        frame: u32,
        flip: bool,
    ) {
        for (slot, row) in ROWS.into_iter().enumerate() {
            let Some(trail) = self.0[slot] else { continue };
            let Some(view) = native_fill(videoram, row).and_then(|fill| trail.view(fill, frame))
            else {
                continue;
            };
            let edge = FIRST_FILL_X + 2 * usize::from(trail.fill);
            let top = row * 8 - VISAREA_Y + 1;
            let mut paint = |x, y, width, height, (color, pen)| {
                let (x, y) = if flip {
                    (WIDTH - x - width, HEIGHT - y - height)
                } else {
                    (x, y)
                };
                edits.fills.push(TextFill {
                    x,
                    y,
                    width,
                    height,
                    color,
                    pen,
                });
            };
            let width = 2 * usize::from(view.trail);
            paint(edge, top, width, 5, view.ink);
            if view.highlight {
                paint(edge, top, width, 1, GOLD);
            }
            if let Some(ink) = view.marker {
                paint(edge, top + 1, 2, 3, ink);
            }
        }
    }

    pub(super) fn ally2_view(&self, fill: u8, frame: u32) -> Option<TrailView> {
        self.0[2]?.view(fill, frame)
    }
}

pub(super) fn heavy_hits(
    impacts: &[ImpactCue],
    right_player_only: bool,
    shared: bool,
) -> [bool; 3] {
    let [p1, p2, ally2] = ImpactFighter::SLOTS.map(|fighter| {
        impacts
            .iter()
            .any(|cue| cue.fighter == fighter && cue.kind == ImpactKind::HeavyStrike)
    });
    // In native one-player Ken Arcade, Ken takes the top bar and the CPU
    // the bottom. Combat cues identify them by the opposite side indices.
    if right_player_only {
        [p2, p1, ally2]
    } else {
        [p1 || (shared && ally2), p2, ally2]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Trail {
    fill: u8,
    lost: u8,
    frame: u32,
    heavy: bool,
}

pub(super) struct TrailView {
    trail: u8,
    ink: (u8, u8),
    highlight: bool,
    marker: Option<(u8, u8)>,
}

impl Trail {
    fn view(self, fill: u8, frame: u32) -> Option<TrailView> {
        let age = frame.wrapping_sub(self.frame);
        let (hold, decay, marker) = if self.heavy { (4, 12, 8) } else { (2, 8, 5) };
        if fill != self.fill || age >= hold + decay {
            return None;
        }
        let time_left = decay - age.saturating_sub(hold);
        let trail = (u32::from(self.lost) * time_left).div_ceil(decay) as u8;
        Some(TrailView {
            trail,
            ink: if age < hold { AMBER } else { SHADOW },
            highlight: self.heavy,
            marker: (age < marker).then_some(if age < 3 { LIGHT } else { EDGE_GOLD }),
        })
    }
}

impl TrailView {
    /// Pens for Ally 2's own bar. Its inner fill is one pixel per point.
    pub(super) fn ally_ink(&self, row: usize, beyond_fill: usize) -> Option<(u8, u8)> {
        if beyond_fill == 1 {
            if let Some(marker) = self.marker {
                return Some(marker);
            }
        }
        (beyond_fill <= usize::from(self.trail)).then_some(if self.highlight && row == 1 {
            GOLD
        } else {
            self.ink
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::Fight;
    use super::*;
    use crate::front::PlayerInputs;

    #[test]
    fn heavy_contact_follows_the_bar_that_owns_the_damage() {
        let cue = |fighter| ImpactCue {
            kind: ImpactKind::HeavyStrike,
            frame: 0,
            fighter,
            x: 0,
            y: 0,
            direction: crate::impact::ImpactDirection::Left,
        };
        let ken = [cue(ImpactFighter::P2)];
        assert_eq!(heavy_hits(&ken, false, false), [false, true, false]);
        assert_eq!(heavy_hits(&ken, true, false), [true, false, false]);
        let ally = [cue(ImpactFighter::Ally2)];
        assert_eq!(heavy_hits(&ally, false, false), [false, false, true]);
        assert_eq!(heavy_hits(&ally, false, true), [true, false, true]);
    }

    #[test]
    fn native_light_and_heavy_hits_paint_only_the_loss_and_replay_through_rollback() {
        let (assets, ready) = crate::impact::tests::close_local_fight();
        let idle = PlayerInputs::default();
        let row = ROWS[1];
        let bar_top = row * 8 - VISAREA_Y;
        for (button, duration, heavy) in [(0, 10, false), (2, 16, true)] {
            let mut fight = ready.clone();
            let mut attack = idle;
            attack.buttons[button] = true;
            let (before, input, previous, fill, frame) = (0..30)
                .find_map(|index| {
                    let before = fight.save_netplay_snapshot(index);
                    let input = if index == 0 { attack } else { idle };
                    let previous = native_fill(&fight.machine.state.videoram, row).unwrap();
                    let stepped = fight.step(&assets, input, idle, false);
                    let fill = native_fill(&fight.machine.state.videoram, row).unwrap();
                    if previous > fill {
                        assert!(fill > 0);
                        assert_eq!(
                            stepped
                                .impacts
                                .iter()
                                .any(|cue| cue.kind == ImpactKind::HeavyStrike),
                            heavy
                        );
                        Some((before, input, previous, fill, stepped.frame))
                    } else {
                        None
                    }
                })
                .expect("the standing attack visibly damages the other fighter");
            let during = fight.save_netplay_snapshot(before.frame + 1);
            let checksum = fight.state_checksum();
            fight.restore_netplay_snapshot(&before);
            assert_eq!(
                fight.step(&assets, input, idle, false).frame.argb,
                frame.argb
            );

            let edge = FIRST_FILL_X + 2 * usize::from(fill);
            let old_edge = FIRST_FILL_X + 2 * usize::from(previous);
            let verify = |fight: &Fight, active: bool, marker: bool, flip: bool| {
                let image = fight.render(&assets);
                let mut plain = fight.clone();
                plain.lifebar_trails = LifebarTrails::default();
                let baseline = plain.render(&assets);
                assert_eq!(fight.state_checksum(), plain.state_checksum());
                let mut differences = 0;
                for (index, (pixel, original)) in image.argb.iter().zip(&baseline.argb).enumerate()
                {
                    if pixel == original {
                        continue;
                    }
                    let (x, y) = (index % WIDTH, index / WIDTH);
                    let (x, y) = if flip {
                        (WIDTH - 1 - x, HEIGHT - 1 - y)
                    } else {
                        (x, y)
                    };
                    assert!(
                        (edge..old_edge).contains(&x) && (bar_top + 1..bar_top + 6).contains(&y)
                    );
                    differences += 1;
                }
                assert_eq!(differences != 0, active);
                let sample = |y| {
                    let (x, y) = if flip {
                        (WIDTH - 1 - edge, HEIGHT - 1 - y)
                    } else {
                        (edge, y)
                    };
                    image.argb[y * WIDTH + x]
                };
                let edge_value = sample(bar_top + 2);
                let fill_value = sample(bar_top + 5);
                assert_eq!(
                    edge_value != fill_value,
                    marker,
                    "marker at native frame {}, with edge {edge_value:08x} and tail {fill_value:08x}",
                    fight.machine.state.frame
                );
            };
            assert_eq!(fight.state_checksum(), checksum);
            verify(&fight, true, true, false);
            let mut mirrored = fight.clone();
            mirrored.machine.state.flip_screen = true;
            verify(&mirrored, true, true, true);
            for age in 1..=duration {
                fight.step(&assets, idle, idle, false);
                verify(
                    &fight,
                    age < duration,
                    age < if heavy { 8 } else { 5 },
                    false,
                );
            }
            fight.restore_netplay_snapshot(&during);
            assert_eq!(
                fight.render(&assets).argb,
                frame.argb,
                "the snapshot includes the live trail"
            );
            assert!(fight.restart_round());
            assert_eq!(fight.lifebar_trails, LifebarTrails::default());
        }
    }

    #[test]
    fn new_loss_replaces_the_tail_and_disappearing_or_refilled_bars_clear() {
        let mut trails = LifebarTrails::default();
        let shown = |life| [Some(life), None, None];
        trails.step(shown(48), shown(40), [false; 3], 1);
        assert_eq!(trails.0[0].unwrap().lost, 8);
        trails.step(shown(40), shown(35), [false; 3], 2);
        assert_eq!(
            trails.0[0].unwrap().lost,
            5,
            "only the latest damage is accented"
        );
        trails.step(shown(35), shown(36), [false; 3], 3);
        assert_eq!(trails, LifebarTrails::default());
        trails.step(shown(36), shown(34), [false; 3], 4);
        trails.step(shown(34), shown(0), [false; 3], 5);
        assert_eq!(trails, LifebarTrails::default());
        trails.step(shown(48), shown(38), [false; 3], 6);
        trails.step(shown(38), [None; 3], [false; 3], 7);
        trails.step([None; 3], shown(38), [false; 3], 8);
        assert_eq!(trails, LifebarTrails::default());
    }
}
