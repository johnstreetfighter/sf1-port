//! Feedback drawn over the selected tile, preview and caption.

use sf_video::render::WIDTH;

use super::Rect;

const CREAM: u32 = 0xffff_ffaa;
const WARM: u32 = 0xffff_dd88;
const GOLD: u32 = 0xffff_bb00;
const SCAN_FRAMES: u32 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Direction {
    Forward,
    Backward,
}

impl Direction {
    /// Replicated picks carry a destination. Follow the shorter route
    /// through the wrapping list when a packet skips intervening tiles.
    pub(super) fn between(from: usize, to: usize, tiles: usize) -> Self {
        let forward = (to + tiles - from) % tiles;
        if forward <= tiles - forward { Self::Forward } else { Self::Backward }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct StageMotion {
    frame: u32,
    direction: Direction,
}

impl StageMotion {
    pub(super) fn new(frame: u32, direction: Direction) -> Self {
        Self { frame, direction }
    }

    pub(super) fn at(self, frame: u32) -> Option<SelectionCue> {
        let age = frame.wrapping_sub(self.frame);
        (age < SCAN_FRAMES).then_some(SelectionCue { age: age as usize, direction: self.direction })
    }
}

#[derive(Clone, Copy)]
pub(super) struct SelectionCue {
    age: usize,
    direction: Direction,
}

impl SelectionCue {
    pub(super) fn preview(self, argb: &mut [u32], (x, y, width, height): Rect) {
        let slide = match self.age {
            0..=1 => 3,
            2..=3 => 2,
            4 => 1,
            _ => 0,
        };
        let travel = width - 5;
        let progress = (self.age * travel + 3) / 7;
        let scan = match self.direction {
            Direction::Forward => progress,
            Direction::Backward => travel - progress,
        };
        for row in y..y + height {
            let line = &mut argb[row * WIDTH + x..][..width];
            if slide != 0 {
                // Extend the edge column through the seam, keeping all art
                // and the Random glyph within the original preview window.
                match self.direction {
                    Direction::Forward => {
                        let edge = line[0];
                        line.copy_within(..width - slide, slide);
                        line[..slide].fill(edge);
                    }
                    Direction::Backward => {
                        let edge = line[width - 1];
                        line.copy_within(slide.., 0);
                        line[width - slide..].fill(edge);
                    }
                }
            }
            for pixel in &mut line[scan..scan + 5] {
                *pixel = lighten(*pixel, [255, 236, 165], 26);
            }
            line[scan + 2] = lighten(line[scan + 2], [255, 255, 208], 37);
        }
        for (row, color) in [(y - 1, CREAM), (y + height, GOLD)] {
            let start = row * WIDTH + x + scan + 1;
            argb[start..start + 3].fill(color);
        }
    }

    pub(super) fn caption_pens(self) -> Option<[u32; 3]> {
        match self.age {
            0..=2 => Some([CREAM, 0xff88_4400, 0xffff_ee99]),
            3..=5 => Some([0xffff_ee99, 0xff77_5500, WARM]),
            _ => None,
        }
    }

    pub(super) fn tile(self, argb: &mut [u32], (x, y, width, height): Rect) {
        if self.age >= 6 {
            return;
        }
        let settle = 2 - self.age / 2;
        let (horizontal, vertical, color) = if self.age < 3 { (8, 5, CREAM) } else { (5, 3, WARM) };
        let right = x + width - 1;
        let bottom = y + height - 1;
        for row in [y, bottom] {
            let line = &mut argb[row * WIDTH..][..WIDTH];
            let left_start = x + settle;
            let right_end = right + 1 - settle;
            line[left_start..left_start + horizontal].fill(color);
            line[right_end - horizontal..right_end].fill(color);
        }
        for row in (y + settle..y + settle + vertical).chain(bottom + 1 - settle - vertical..bottom + 1 - settle) {
            argb[row * WIDTH + x] = color;
            argb[row * WIDTH + right] = color;
        }
    }
}

fn lighten(pixel: u32, targets: [u32; 3], amount: u32) -> u32 {
    [16, 8, 0].into_iter().zip(targets).fold(pixel & 0xff00_0000, |argb, (shift, target)| {
        let value = (pixel >> shift) & 0xff;
        argb | (((value * (100 - amount) + target * amount + 50) / 100) << shift)
    })
}
