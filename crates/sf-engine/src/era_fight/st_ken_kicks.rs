use super::{Dir, MotionTracker};

pub(crate) const KAMA_BARAI: u8 = 0x2a;
pub(crate) const NATA_OTOSHI: u8 = 0x2b;
pub(crate) const OSOTO_MAWASHI: u8 = 0x2c;

use Dir::{Back, Down, DownBack, DownForward, Forward};
pub(crate) const KAMA_MOTION: &[Dir] = &[Down, DownForward, Forward];
pub(crate) const NATA_MOTION: &[Dir] = &[Forward, DownForward, Down];
pub(crate) const OSOTO_MOTION: &[Dir] = &[Back, DownBack, Down, DownForward, Forward];

// The World ROM checks half circle, quarter circle, and reverse quarter
// circle in this order, interleaved with the other special recognizers.
pub(super) const MOTIONS: [(u8, &[Dir]); 3] = [
    (OSOTO_MAWASHI, OSOTO_MOTION),
    (KAMA_BARAI, KAMA_MOTION),
    (NATA_OTOSHI, NATA_MOTION),
];

pub(crate) fn slot(move_id: u8) -> bool {
    matches!(move_id, KAMA_BARAI | NATA_OTOSHI | OSOTO_MAWASHI)
}

const STAGE_WINDOWS: [u8; 32] = [
    10, 8, 8, 11, 8, 8, 9, 8, 9, 8, 8, 10, 8, 8, 11, 13, 8, 15, 9, 8, 10, 12, 8, 8, 11, 8, 9, 8,
    14, 12, 8, 10,
];
pub(super) const FINISH_WINDOW: u8 = 12;

impl MotionTracker {
    pub(super) fn step_st_ken_kick(&mut self, direction: u8, pattern: &[Dir], random: u8) {
        if self.position != 0 {
            self.remaining -= 1;
            if self.remaining == 0 {
                self.position = 0;
                return;
            }
        }
        if pattern
            .get(usize::from(self.position))
            .map(|dir| dir.bits())
            == Some(direction)
        {
            self.position += 1;
            self.remaining = if self.completed(pattern) {
                FINISH_WINDOW
            } else {
                STAGE_WINDOWS[usize::from(random & 31)]
            };
        }
    }
}
