//! Presentation for the last ten counts of the native round clock.

use sf_video::render::TextInkTile;

use super::{
    text_code, BLANK_TILE, TEXT_COLS, TIME_COLS, TIME_ROW, UNDER_TIME_COLS, UNDER_TIME_ROWS,
};

const DIGIT_START: usize = 5;
const AMBER: [u32; 3] = [0xffff_eeaa, 0xff88_5500, 0xffff_cc77];
const CREAM: [u32; 3] = [0xffff_ffaa, 0xffaa_0000, 0xffff_ffaa];

/// Read the digits the native text writer has already drawn. A partial wipe,
/// a replacement message, or another native screen leaves them unaccented.
pub(super) fn digits(videoram: &[u8]) -> Option<[u8; 3]> {
    let cell = |col, row| {
        let offset = (row * TEXT_COLS + col) * 2;
        u16::from_be_bytes([videoram[offset], videoram[offset + 1]])
    };
    if !b"TIME"
        .iter()
        .copied()
        .enumerate()
        .all(|(index, letter)| cell(TIME_COLS.0 + index, TIME_ROW) & 0x3ff == text_code(letter))
        || cell(TIME_COLS.0 + DIGIT_START - 1, TIME_ROW) & 0x3ff != BLANK_TILE
        || (UNDER_TIME_ROWS.0..=UNDER_TIME_ROWS.1).any(|row| {
            (UNDER_TIME_COLS.0..=UNDER_TIME_COLS.1).any(|col| cell(col, row) & 0x3ff != BLANK_TILE)
        })
    {
        return None;
    }
    let words = [0, 1, 2].map(|digit| cell(TIME_COLS.0 + DIGIT_START + digit, TIME_ROW));
    words
        .iter()
        .all(|word| matches!(word, 0x3000..=0x3009))
        .then(|| words.map(|word| (word & 0xf) as u8))
}

fn count([hundreds, tens, ones]: [u8; 3]) -> u16 {
    100 * u16::from(hundreds) + 10 * u16::from(tens) + u16::from(ones)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct TimerMotion {
    tick: Option<Tick>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tick {
    frame: u32,
    changed: [bool; 3],
}

impl TimerMotion {
    pub(super) fn step(&mut self, before: Option<[u8; 3]>, after: Option<[u8; 3]>, frame: u32) {
        let Some((before, after)) = before
            .zip(after)
            .filter(|&(_, after)| (1..=10).contains(&count(after)))
        else {
            self.tick = None;
            return;
        };
        if count(before) == count(after) + 1 {
            self.tick = Some(Tick {
                frame,
                changed: [0, 1, 2].map(|digit| before[digit] != after[digit]),
            });
        } else if before != after
            || self
                .tick
                .is_some_and(|tick| frame.wrapping_sub(tick.frame) >= 5)
        {
            self.tick = None;
        }
    }

    pub(super) fn accent(self, digits: Option<[u8; 3]>, frame: u32) -> Option<CountdownAccent> {
        let value = count(digits?);
        if !(1..=10).contains(&value) {
            return None;
        }
        let (changed, offset) = match self
            .tick
            .map(|tick| (tick.changed, frame.wrapping_sub(tick.frame)))
        {
            Some((changed, 0..=2)) => (changed, -1),
            Some((changed, 3..=4)) => (changed, 1),
            _ => ([false; 3], 0),
        };
        Some(CountdownAccent { changed, offset })
    }
}

#[derive(Clone, Copy)]
pub(super) struct CountdownAccent {
    changed: [bool; 3],
    offset: i32,
}

impl CountdownAccent {
    pub(super) fn tile(self, index: usize, (x, y, word): (i32, i32, u16)) -> Option<TextInkTile> {
        let changed = *self.changed.get(index.checked_sub(DIGIT_START)?)?;
        let offset = if changed { self.offset } else { 0 };
        let inks = if offset < 0 { CREAM } else { AMBER };
        Some(TextInkTile {
            x,
            y: y + offset,
            word,
            scale: 1,
            inks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_countdown_tick_moves_only_its_changed_digits_for_five_frames() {
        let mut timer = TimerMotion::default();
        timer.step(Some([0, 1, 1]), Some([0, 1, 0]), 90);
        let tile = |timer: TimerMotion, frame, index| {
            timer
                .accent(Some([0, 1, 0]), frame)
                .unwrap()
                .tile(index, (204, 52, 0x3000))
        };
        assert!(
            tile(timer, 90, 4).is_none(),
            "the TIME label is never recolored"
        );
        for (age, y, inks) in [
            (0, 51, CREAM),
            (1, 51, CREAM),
            (2, 51, CREAM),
            (3, 53, AMBER),
            (4, 53, AMBER),
            (5, 52, AMBER),
        ] {
            let changed = tile(timer, 90 + age, 7).unwrap();
            assert_eq!((changed.y, changed.inks), (y, inks));
            for index in [5, 6] {
                let unchanged = tile(timer, 90 + age, index).unwrap();
                assert_eq!((unchanged.y, unchanged.inks), (52, AMBER));
            }
        }
        timer.step(Some([0, 1, 0]), Some([0, 0, 9]), 150);
        let digits = timer.accent(Some([0, 0, 9]), 150).unwrap();
        assert_eq!(
            [5, 6, 7].map(|i| digits.tile(i, (204, 52, 0x3000)).unwrap().y),
            [52, 51, 51]
        );
    }

    #[test]
    fn the_clock_can_return_or_reset_without_replaying_a_tick() {
        let mut timer = TimerMotion::default();
        timer.step(Some([0, 0, 2]), Some([0, 0, 1]), u32::MAX);
        let ones = |timer: TimerMotion, frame| {
            timer
                .accent(Some([0, 0, 1]), frame)
                .unwrap()
                .tile(7, (220, 52, 0x3001))
                .unwrap()
        };
        assert_eq!(ones(timer, 0).y, 51);
        timer.step(Some([0, 0, 1]), None, 0);
        timer.step(None, Some([0, 0, 1]), 1);
        assert_eq!(ones(timer, 1).y, 52);
        timer.step(Some([0, 0, 3]), Some([0, 0, 1]), 2);
        assert_eq!(ones(timer, 2).y, 52);
        assert!(timer.accent(Some([0, 1, 1]), 2).is_none());
        assert!(timer.accent(Some([0, 0, 0]), 2).is_none());
    }

    #[test]
    fn visible_rom_ticks_drive_the_timer_and_restore_identically() {
        use crate::front::cabinet::flow_position;
        use crate::front::PlayerInputs;
        use crate::tasks::TIME_DIGITS;
        use sf_video::compositor::RomFont;
        use sf_video::render::{Frame, WIDTH};

        let (assets, mut fight) = crate::impact::tests::close_local_fight();
        let offset = (TIME_DIGITS - 0xff8000) as usize;
        let seed_timer = |fight: &mut super::super::Fight, digits: [u8; 3]| {
            let mut before = fight.save_netplay_snapshot(0);
            before.machine.work[offset..offset + 3].copy_from_slice(&digits);
            before.machine.work[offset + 3] = 59;
            for (digit, value) in digits.into_iter().enumerate() {
                let at = (TIME_ROW * TEXT_COLS + TIME_COLS.0 + DIGIT_START + digit) * 2;
                before.machine.state.videoram[at..at + 2].copy_from_slice(&[0x30, value]);
            }
            fight.restore_netplay_snapshot(&before);
        };
        seed_timer(&mut fight, [0, 1, 1]);
        assert_eq!(digits(&fight.machine().state.videoram), Some([0, 1, 1]));
        let idle = PlayerInputs::default();
        let mut first_tick = None;
        for _ in 0..3 {
            let snapshot = fight.save_netplay_snapshot(0);
            let stepped = fight.step(&assets, idle, idle, false);
            if digits(&fight.machine().state.videoram) == Some([0, 1, 0]) {
                first_tick = Some((snapshot, stepped.frame));
                break;
            }
        }
        let (before_tick, frame) = first_tick.expect("the native clock visibly reaches ten");
        let tick_frame = fight.machine().state.frame;
        let font = RomFont::new(&assets.gfx4, &assets.maincpu);
        let expected = |native: &super::super::Fight, age: u32| -> Frame {
            let scene = native.machine().state.scene();
            let mut edits = native.layout(&scene).unwrap();
            assert_eq!(edits.inked.len(), 3);
            edits.inked.clear();
            let mut base = sf_video::render::render_frame_edited(&assets, &scene, &edits);
            let recoil = match age {
                0..=2 => (51, CREAM),
                3..=4 => (53, AMBER),
                _ => (52, AMBER),
            };
            for (index, ch) in [0, 1, 0].into_iter().enumerate() {
                let (y, palette) = if index == 2 { recoil } else { (52, AMBER) };
                font.tile_in(&mut base.argb, 204 + index * 8, y, ch, palette);
            }
            base
        };
        let check_timer_region = |a: &Frame, b: &Frame| {
            for y in 51..61 {
                let range = y * WIDTH + 164..y * WIDTH + 228;
                assert_eq!(a.argb[range.clone()], b.argb[range]);
            }
        };
        check_timer_region(&frame, &expected(&fight, 0));
        fight.restore_netplay_snapshot(&before_tick);
        assert_eq!(
            fight.step(&assets, idle, idle, false).frame.argb,
            frame.argb
        );
        let during = fight.save_netplay_snapshot(1);
        for age in 1..=5 {
            let actual = fight.step(&assets, idle, idle, false).frame;
            assert_eq!(fight.machine().state.frame, tick_frame.wrapping_add(age));
            check_timer_region(&actual, &expected(&fight, age));
        }
        fight.restore_netplay_snapshot(&during);
        assert_eq!(fight.render(&assets).argb, frame.argb);

        seed_timer(&mut fight, [0, 0, 1]);
        let mut timed_out = false;
        for _ in 0..8 {
            fight.step(&assets, idle, idle, false);
            let scene = fight.machine().state.scene();
            if flow_position(fight.machine()) == 0x3ed96 {
                assert_eq!(&fight.machine().work_ram()[offset..offset + 3], &[0; 3]);
                let edits = fight.layout(&scene).unwrap();
                assert!(edits.inked.is_empty(), "TIME UP receives no urgency pixels");
                timed_out = true;
                break;
            }
        }
        assert!(timed_out, "the native 001→000 tick reaches TIME UP");
    }
}
