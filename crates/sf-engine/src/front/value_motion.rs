//! Brief value feedback on the front-end clock, shared by Settings and Pause.

use std::ops::Range;

use sf_video::compositor::{ink, RomFont};
use sf_video::render::WIDTH;

use crate::roster::MenuInput;

const FLASH_FRAMES: u32 = 6;
const TICK_FRAMES: u32 = 7;
const CREAM: [u32; 3] = [0xffff_ffaa, 0xff88_4400, 0xffff_de80];
const WARM: [u32; 3] = [0xffff_d477, 0xff88_4400, 0xffff_aa33];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ValueDirection {
    Left,
    Right,
}

impl ValueDirection {
    pub(super) fn from_input(input: MenuInput) -> Option<Self> {
        match input {
            MenuInput::Left => Some(Self::Left),
            MenuInput::Right => Some(Self::Right),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ValueMotion {
    frame: u32,
    direction: Option<ValueDirection>,
}

impl ValueMotion {
    pub(super) fn new(frame: u32, direction: Option<ValueDirection>) -> Self {
        Self { frame, direction }
    }

    pub(super) fn at(self, frame: u32) -> Option<ValueCue> {
        let age = frame.wrapping_sub(self.frame);
        let duration = if self.direction.is_some() { TICK_FRAMES } else { FLASH_FRAMES };
        (age < duration).then_some(ValueCue { age, direction: self.direction })
    }
}

#[derive(Clone, Copy)]
pub(super) struct ValueCue {
    age: u32,
    direction: Option<ValueDirection>,
}

impl ValueCue {
    pub(super) fn draw(
        self, argb: &mut [u32], font: &RomFont<'_>, value: &str, (x, y): (usize, usize), clip: Range<usize>,
    ) {
        let width = value.chars().count() * 8;
        let offset = match (self.direction, self.age) {
            (Some(ValueDirection::Left), 0..=1) | (Some(ValueDirection::Right), 2..=4) => -1,
            (Some(ValueDirection::Right), 0..=1) | (Some(ValueDirection::Left), 2..=4) => 1,
            _ => 0,
        } * if self.age < 2 { 2 } else { 1 };
        let rightmost = x.max(clip.end.saturating_sub(width));
        let drawn_x = x.saturating_add_signed(offset).clamp(clip.start.min(x), rightmost);
        if self.age < FLASH_FRAMES {
            let pens = if self.age < 3 { CREAM } else { WARM };
            for (i, ch) in value.chars().enumerate() {
                font.glyph_in(argb, drawn_x + 8 * i, y, ch, pens);
            }
        } else {
            font.text_px(argb, x, y, value, ink::YELLOW);
        }
        if let Some(direction) = self.direction {
            self.tick(argb, x..x + width, drawn_x..drawn_x + width, y + 3, clip, direction);
        }
    }

    fn tick(
        self, argb: &mut [u32], resting: Range<usize>, drawn: Range<usize>, y: usize,
        clip: Range<usize>, direction: ValueDirection,
    ) {
        let span = if self.age < 4 { 5 } else { 3 };
        let travel = self.age as usize / 2;
        let color = if self.age < 4 { 0xffff_ffaa } else { 0xffdd_8800 };
        let left = match direction {
            ValueDirection::Left => resting.start.saturating_sub(span + 1 + travel),
            ValueDirection::Right => resting.end + 2 + travel,
        };
        for i in 0..span {
            let x = left + match direction {
                ValueDirection::Left => span - 1 - i,
                ValueDirection::Right => i,
            };
            if clip.contains(&x) && !drawn.contains(&x) {
                argb[y * WIDTH + x] = color;
                if matches!(i, 1 | 2) {
                    argb[(y - i) * WIDTH + x] = color;
                    argb[(y + i) * WIDTH + x] = color;
                }
            }
        }
    }
}
