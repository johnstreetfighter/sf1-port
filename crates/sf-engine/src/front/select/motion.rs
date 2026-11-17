//! Short feedback for the current portrait and its grid cursor.

use sf_video::compositor::SelectPortrait;
use sf_video::render::WIDTH;

use super::StyleLatch;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SideMotion {
    pub moved_at: Option<u32>,
    pub latch: Option<StyleLatch>,
}

#[derive(Clone, Copy)]
pub(super) struct MoveCue(u32);

impl MoveCue {
    pub(super) fn read(start: Option<u32>, frame: u32) -> Option<Self> {
        let age = frame.wrapping_sub(start?);
        (age < 5).then_some(Self(age))
    }

    pub(super) fn settle(self) -> usize {
        if self.0 < 2 { 2 } else { 1 }
    }

    pub(super) fn bright(self) -> bool {
        self.0 < 3
    }

    pub(super) fn portrait(
        self, argb: &mut [u32], area: (usize, usize, usize, usize), portrait: &SelectPortrait<'_>,
        (sx, sy): (isize, isize), side: usize,
    ) {
        let (x, y, w, h) = area;
        let colors = if side == 0 { [233, 243, 255] } else { [255, 228, 200] };
        let pct = if self.0 < 2 { 34 } else { 17 };
        // Move each panel's art toward its outside edge. The panel and its
        // label are at their final coordinates even on the first frame.
        let offset = self.settle() as isize * if side == 0 { -1 } else { 1 };
        for dy in 0..h {
            let src_y = (sy + dy as isize) as usize;
            if src_y >= portrait.height {
                continue;
            }
            let src_row = src_y * portrait.width;
            let Some(source) = portrait.argb.get(src_row..src_row + portrait.width) else {
                continue;
            };
            let dest = &mut argb[(y + dy) * WIDTH + x..][..w];
            for (dx, pixel) in dest.iter_mut().enumerate() {
                let src_x = (sx + dx as isize - offset) as usize;
                if let Some(&color) = source.get(src_x) {
                    *pixel = brighten(color, colors, pct);
                }
            }
        }
    }
}

fn brighten(color: u32, targets: [u32; 3], pct: u32) -> u32 {
    let rgb = [16, 8, 0].map(|shift| color >> shift & 0xff);
    // Preserve the dark backdrop and the portrait's dark outlines.
    if rgb.into_iter().all(|value| value < 64) {
        return color;
    }
    [16, 8, 0].into_iter().zip(rgb).zip(targets).fold(color & 0xff00_0000, |pixel, ((shift, value), target)| {
        let accent = (target.saturating_sub(value) * pct + 50) / 100;
        pixel | ((value + accent) << shift)
    })
}
