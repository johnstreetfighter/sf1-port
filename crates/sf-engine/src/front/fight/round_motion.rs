//! Presentation cues from native round outcomes and their HUD cells.

use sf_video::compositor::header_glyph;
use sf_video::decode::decode_char_tile;
use sf_video::render::{TextEdits, TextFill, TextInkTile, TextSpan, HEIGHT, VISAREA_X, VISAREA_Y, WIDTH};
use sf_video::romset::Assets;
use sf_video::scene::SceneState;

use super::{lifebar, text_code, BLANK_TILE, LABELS, LABEL_COL, LABEL_CELLS, TEXT_COLS};

const KO_FRAMES: u32 = 18;
const MARKER_FRAMES: u32 = 8;
const OFFSETS: [i32; 6] = [3, -3, 2, -2, 1, -1];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct RoundMotion {
    knockout: Option<Knockout>,
    markers: [Option<MarkerCue>; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Knockout {
    starts: u32,
    emptied: [bool; 2],
    double: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MarkerCue {
    starts: u32,
    offset: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct HudRound {
    pub knockout: Option<HudKo>,
    pub markers: [Option<HudWins>; 2],
}

#[derive(Clone, Copy, Debug)]
pub(super) struct HudKo {
    emptied: [bool; 2],
    double: bool,
}

impl HudKo {
    pub(super) fn read(native_empty: [bool; 2], dramatic_health: Option<[u16; 3]>) -> Self {
        let Some([ally1, boss, ally2]) = dramatic_health else {
            return Self { emptied: native_empty, double: native_empty == [true; 2] };
        };
        let team_down = ally1 == 0 && ally2 == 0;
        let boss_down = boss == 0;
        Self {
            emptied: [native_empty[0] && team_down, native_empty[1] && boss_down],
            double: team_down && boss_down,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct HudWins {
    pub count: u8,
    pub base: usize,
    pub visible: u8,
}

impl HudWins {
    pub(super) fn read(count: u8, base: usize, videoram: &[u8]) -> Self {
        let visible = (0..2)
            .take_while(|slot| marker_present(videoram, base + 4 * slot))
            .count() as u8;
        Self { count, base, visible }
    }
}

impl RoundMotion {
    pub(super) fn step(&mut self, before: HudRound, after: HudRound, frame: u32) {
        if let Some(ko) = after.knockout.filter(|_| before.knockout.is_none()) {
            self.knockout = Some(Knockout {
                starts: frame,
                emptied: ko.emptied,
                double: ko.double,
            });
        } else if self.knockout.is_some_and(|ko| frame.wrapping_sub(ko.starts) >= KO_FRAMES)
            || after.markers.iter().all(Option::is_none)
        {
            self.knockout = None;
        }
        for (index, cue) in self.markers.iter_mut().enumerate() {
            let (Some(previous), Some(current)) = (before.markers[index], after.markers[index]) else {
                *cue = None;
                continue;
            };
            // The native win counter distinguishes an award from red markers
            // being drawn again when a new round's HUD opens.
            if previous.base == current.base && current.count > previous.count
                && current.count <= current.visible
            {
                *cue = Some(MarkerCue {
                    starts: frame,
                    offset: current.base + 4 * usize::from(current.count - 1),
                });
            } else if cue.is_some_and(|cue| frame.wrapping_sub(cue.starts) >= MARKER_FRAMES)
                || previous.base != current.base || current.count < previous.count
            {
                *cue = None;
            }
        }
    }

    pub(super) fn paint(
        &self, assets: &Assets, edits: &mut TextEdits, scene: &SceneState,
        frame: u32, shake: bool,
    ) {
        if scene.active & 0x08 == 0 {
            return;
        }
        if let Some(ko) = self.knockout {
            let age = frame.wrapping_sub(ko.starts);
            if age < 14 {
                for (row, empty) in lifebar::ROWS.into_iter().zip(ko.emptied) {
                    if empty && lifebar::native_fill(&scene.videoram, row) == Some(0) {
                        let x = KO_BAR_LEFT as i32;
                        let y = (row * 8 - VISAREA_Y) as i32;
                        let dx = if shake && !ko.double {
                            OFFSETS.get((age / 2) as usize).copied().unwrap_or_default()
                        } else {
                            0
                        };
                        let dx = if dx != 0 && shift_label(edits, scene, row, dx) {
                            shift_cells(edits, scene, row, lifebar::FIRST_COL, lifebar::FILL_CELLS + 2, dx);
                            dx
                        } else {
                            0
                        };
                        flash_bar(edits, scene, x + dx, y, age);
                    }
                }
            }
            if age < KO_FRAMES && center_clear(assets, scene) {
                stamp(edits, age);
            }
        }
        for cue in self.markers.iter().flatten() {
            let age = frame.wrapping_sub(cue.starts);
            if age < MARKER_FRAMES && marker_present(&scene.videoram, cue.offset) {
                marker_glint(assets, edits, scene, cue.offset, age);
            }
        }
    }
}

const KO_BAR_LEFT: usize = (lifebar::FIRST_COL + 1) * 8 - VISAREA_X - 2;
const KO_BAR_WIDTH: i32 = (lifebar::FILL_CELLS * 8 + 2) as i32;

fn text_word(videoram: &[u8], col: usize, row: usize) -> u16 {
    let i = 2 * (row * TEXT_COLS + col);
    u16::from_be_bytes([videoram[i], videoram[i + 1]])
}

fn marker_present(videoram: &[u8], offset: usize) -> bool {
    let (col, row) = (offset % 0x80 / 2, offset / 0x80);
    offset % 2 == 0 && col < TEXT_COLS - 1 && row < 31
        && [0x1100, 0x1101, 0x1110, 0x1111].into_iter().enumerate().all(|(i, word)| {
            text_word(videoram, col + i % 2, row + i / 2) == word
        })
}

fn screen_position(scene: &SceneState, x: i32, y: i32, width: i32, height: i32) -> (i32, i32) {
    if scene.flip_screen { (WIDTH as i32 - x - width, HEIGHT as i32 - y - height) } else { (x, y) }
}

fn shift_label(edits: &mut TextEdits, scene: &SceneState, row: usize, dx: i32) -> bool {
    let mut changed = false;
    let y = (row * 8 - VISAREA_Y) as i32;
    let x = (lifebar::FIRST_COL * 8 - VISAREA_X) as i32;
    for tile in edits.added.iter_mut().filter(|tile| tile.1 == y && tile.0 < x) {
        tile.0 += dx;
        changed = true;
    }
    if changed {
        return true;
    }
    // An unprojected label (such as PLAYER2 at the top in Ken Arcade)
    // still moves with its bar in its original character art.
    if LABELS.into_iter().any(|(_, label, _)| {
        label.bytes().enumerate().all(|(i, b)| {
            text_word(&scene.videoram, LABEL_COL + i, row) & 0x3ff == text_code(b)
        })
    }) {
        shift_cells(edits, scene, row, LABEL_COL, LABEL_CELLS, dx);
        true
    } else {
        false
    }
}

fn shift_cells(edits: &mut TextEdits, scene: &SceneState, row: usize, first: usize, count: usize, dx: i32) {
    edits.hidden.push(TextSpan { row, col_first: first, col_last: first + count - 1 });
    for col in first..first + count {
        let x = (col * 8 - VISAREA_X) as i32;
        let y = (row * 8 - VISAREA_Y) as i32;
        let (x, y) = screen_position(scene, x + dx, y, 8, 8);
        let word = text_word(&scene.videoram, col, row) ^ if scene.flip_screen { 0xc00 } else { 0 };
        edits.added.push((x, y, word));
    }
}

fn flash_bar(edits: &mut TextEdits, scene: &SceneState, x: i32, y: i32, age: u32) {
    let width = (10 + 12 * age as i32).min(KO_BAR_WIDTH);
    let right = x + KO_BAR_WIDTH - 1;
    let (color, pen) = match age {
        0..=3 => (13, 0),
        4..=7 => (4, 0),
        _ => (4, 2),
    };
    for (x, y, width, height) in [(right + 1 - width, y, width, 1), (right + 1 - width, y + 6, width, 1), (right, y, 1, 7)] {
        let (x, y) = screen_position(scene, x, y, width, height);
        edits.fills.push(TextFill { x: x as usize, y: y as usize, width: width as usize, height: height as usize, color, pen });
    }
}

fn center_clear(assets: &Assets, scene: &SceneState) -> bool {
    // The stamp's 2x pixels run from x141..240 and y74..106. Give
    // native lettering or the round wipe first use of those text cells.
    let (x, y) = screen_position(scene, 141, 74, 99, 32);
    let col = (x as usize + VISAREA_X) / 8;
    let col_last = (x as usize + VISAREA_X + 98) / 8;
    let row = (y as usize + VISAREA_Y) / 8;
    let row_last = (y as usize + VISAREA_Y + 31) / 8;
    (row..=row_last).all(|row| (col..=col_last).all(|col| {
        let code = text_word(&scene.videoram, col, row) & 0x3ff;
        code == BLANK_TILE || decode_char_tile(&assets.gfx4, usize::from(code)).iter().flatten().all(|&pen| pen == 3)
    }))
}

fn stamp(edits: &mut TextEdits, age: u32) {
    let inks = if age < 4 { [0xffff_ffaa, 0xffaa_0000, 0xffff_cc77] } else { [0xffee_cc66, 0xffaa_0000, 0xffee_aa00] };
    let offset = match age { 0..=1 => -4, 2..=3 => 2, _ => 0 };
    for (letter, left) in [('K', 145), ('.', 160), ('O', 191), ('.', 206)] {
        let code = header_glyph(letter).expect("K.O. uses the native header font");
        for (i, code) in [code, code + 1, code + 0x10, code + 0x11].into_iter().enumerate() {
            edits.inked.push(TextInkTile {
                x: left + offset + 16 * (i % 2) as i32,
                y: 74 + 16 * (i / 2) as i32,
                word: code, scale: 2, inks,
            });
        }
    }
}

fn marker_glint(assets: &Assets, edits: &mut TextEdits, scene: &SceneState, offset: usize, age: u32) {
    let (col, row) = (offset % 0x80 / 2, offset / 0x80);
    let column = 2 * age as usize;
    for i in 0..4 {
        let (col, row) = (col + i % 2, row + i / 2);
        let code = text_word(&scene.videoram, col, row) & 0x3ff;
        let tile = decode_char_tile(&assets.gfx4, usize::from(code));
        for (ty, pens) in tile.iter().enumerate() {
            for (tx, &pen) in pens.iter().enumerate() {
                let (x, y) = (8 * (i % 2) + tx, 8 * (i / 2) + ty);
                let ink = match pen {
                    1 if age < 3 => (4, 0),
                    1 if age < 6 => (12, 2),
                    2 if (x == 0 || y <= 1) && (column..column + 4).contains(&(x + y)) => {
                        if age < 4 { (13, 0) } else { (5, 0) }
                    }
                    _ => continue,
                };
                let x = (col * 8 + tx) as i32 - VISAREA_X as i32;
                let y = (row * 8 + ty) as i32 - VISAREA_Y as i32;
                let (x, y) = screen_position(scene, x, y, 1, 1);
                if (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y) {
                    edits.fills.push(TextFill { x: x as usize, y: y as usize, width: 1, height: 1, color: ink.0, pen: ink.1 });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fighter::{P1_STRUCT, P2_STRUCT, WORK_BASE};
    use crate::front::PlayerInputs;
    use crate::tasks::TIME_DIGITS;

    const IDLE: PlayerInputs = PlayerInputs { directions: [false; 4], buttons: [false; 6] };

    fn step(fight: &mut super::super::Fight, assets: &Assets) -> sf_video::render::Frame {
        fight.step(assets, IDLE, IDLE, false).frame
    }

    fn plain_picture(fight: &super::super::Fight, assets: &Assets) -> sf_video::render::Frame {
        let mut plain = fight.clone();
        plain.round_motion = RoundMotion::default();
        assert_eq!(fight.state_checksum(), plain.state_checksum());
        plain.render(assets)
    }

    fn changed<'a>(image: &'a sf_video::render::Frame, plain: &'a sf_video::render::Frame) -> impl Iterator<Item = (usize, usize)> + 'a {
        image.argb.iter().zip(&plain.argb).enumerate().filter_map(|(i, (a, b))| {
            (a != b).then_some((i % WIDTH, i / WIDTH))
        })
    }

    #[test]
    fn actual_knockouts_stamp_and_shake_only_the_loser_and_restore_with_rollback() {
        let (assets, ready) = crate::impact::tests::close_local_fight();
        for (loser, player) in [P1_STRUCT, P2_STRUCT].into_iter().enumerate() {
            let mut fight = ready.clone();
            let mut seed = fight.save_netplay_snapshot(0);
            let at = (player + 6 - WORK_BASE) as usize;
            seed.machine.work[at..at + 2].copy_from_slice(&[1, 0]);
            seed.machine.toggle_state.era.fighters[loser].sub_hp = 20;
            fight.restore_netplay_snapshot(&seed);
            let mut heavy = IDLE;
            heavy.buttons[2] = true;
            let [p1, p2] = if loser == 0 { [IDLE, heavy] } else { [heavy, IDLE] };
            fight.step(&assets, p1, p2, false);
            let (pre_ko, image) = (0..90).find_map(|i| {
                let before = fight.save_netplay_snapshot(i);
                let image = step(&mut fight, &assets);
                fight.round_motion.knockout.is_some().then_some((before, image))
            }).expect("native fight enters K.O. after the heavy attack");
            let row = lifebar::ROWS[loser];
            let top = row * 8 - VISAREA_Y;
            let mut hud = 0;
            let mut stamp = 0;
            let plain = plain_picture(&fight, &assets);
            for (x, y) in changed(&image, &plain) {
                if (top..top + 8).contains(&y) {
                    assert!((109..283).contains(&x));
                    hud += 1;
                } else {
                    assert!((141..240).contains(&x) && (74..106).contains(&y));
                    stamp += 1;
                }
            }
            assert!(hud > 0 && stamp > 0);
            assert_eq!(lifebar::native_fill(&fight.machine.state.videoram, row), Some(0));
            let mut steady = fight.clone();
            steady.set_screen_shake(false);
            let steady = steady.render(&assets);
            assert!(changed(&image, &steady).any(|(x, y)| (109..168).contains(&x) && (top..top + 8).contains(&y)));
            assert!(changed(&steady, &plain).any(|(_, y)| y == top));
            assert!(changed(&steady, &plain).any(|(_, y)| y >= 74));
            let during = fight.save_netplay_snapshot(pre_ko.frame + 1);
            fight.restore_netplay_snapshot(&pre_ko);
            assert_eq!(step(&mut fight, &assets).argb, image.argb);
            assert_eq!(fight.save_netplay_snapshot(during.frame), during);
            assert_eq!(fight.render(&assets).argb, image.argb);
            assert!(fight.restart_round());
            assert_eq!(fight.round_motion, RoundMotion::default());
        }
    }

    #[test]
    fn native_timeout_only_accents_the_newly_awarded_win_and_clears_before_next_round() {
        let (assets, mut fight) = crate::impact::tests::close_local_fight();
        let mut seed = fight.save_netplay_snapshot(0);
        let health = (P2_STRUCT + 6 - WORK_BASE) as usize;
        seed.machine.work[health..health + 2].copy_from_slice(&[1, 0]);
        seed.machine.toggle_state.era.fighters[1].sub_hp = 20;
        let time = (TIME_DIGITS - WORK_BASE) as usize;
        seed.machine.work[time..time + 3].fill(0);
        let winner = (P1_STRUCT - WORK_BASE) as usize;
        let base = usize::from(u16::from_be_bytes([seed.machine.work[winner + 0xc], seed.machine.work[winner + 0xd]]));
        fight.restore_netplay_snapshot(&seed);
        let (x, y) = (base % 0x80 / 2 * 8 - VISAREA_X, base / 0x80 * 8 - VISAREA_Y);
        let mut award = None;
        let mut returned = false;
        for i in 0..900 {
            let image = step(&mut fight, &assets);
            assert!(fight.round_motion.knockout.is_none(), "TIME UP is not a knockout");
            let live = fight.round_motion.markers[0];
            if award.is_none() && live.is_some() {
                award = Some(i);
                assert!(marker_present(&fight.machine.state.videoram, base));
            }
            if let Some(first) = award {
                let plain = plain_picture(&fight, &assets);
                let mut difference = changed(&image, &plain).peekable();
                assert_eq!(difference.peek().is_some(), i - first < 8);
                assert!(difference.all(|(px, py)| (x..x + 16).contains(&px) && (y..y + 16).contains(&py)));
                if fight.last_flow == super::super::FIGHT_LOOP {
                    assert!(marker_present(&fight.machine.state.videoram, base));
                    returned = true;
                    break;
                }
            }
        }
        assert!(award.is_some() && returned);
    }

    #[test]
    fn double_knockout_keeps_both_labels_steady_and_dramatic_uses_team_health() {
        let double_ko = HudKo::read([true, true], None);
        assert!(double_ko.double);
        assert_eq!(double_ko.emptied, [true, true]);
        let surviving_ally = HudKo::read([true, true], Some([0, 0, 400]));
        assert!(!surviving_ally.double);
        assert_eq!(surviving_ally.emptied, [false, true]);
        let all_down = HudKo::read([true, true], Some([0, 0, 0]));
        assert!(all_down.double);
        assert_eq!(all_down.emptied, [true, true]);
    }
}
