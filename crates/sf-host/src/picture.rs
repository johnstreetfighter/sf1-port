//! Where the picture goes on the window (ticket #206, VO3; ADR-0010): the
//! 384×224 game frame at a whole-number vertical scale, its width set by the
//! aspect, centred on the surface. Pure arithmetic, so every look draws into
//! the same rectangle.

use sf_engine::front::settings::{Aspect, MAX_SCALE};
use sf_video::render::{HEIGHT, WIDTH};

/// What a window's frame (its title bar) and the taskbar are allowed to
/// take of a monitor's height when the largest window scale is found. The
/// window library gives the monitor's size, not its work area.
const WORK_AREA_MARGIN: u32 = 120;

/// The largest windowed vertical scale that fits `monitor` (its size in
/// pixels) in `aspect`, 1 at least; the old range's 4 when the monitor does
/// not say. The Video tab's `WINDOW SCALE` runs from 1 to this.
pub fn fit_scale(monitor: Option<(u32, u32)>, aspect: Aspect) -> u32 {
    let Some((w, h)) = monitor else { return 4 };
    (1..=u32::from(MAX_SCALE))
        .rev()
        .find(|&s| picture_width(s, aspect) <= w && HEIGHT as u32 * s + WORK_AREA_MARGIN <= h)
        .unwrap_or(1)
}

/// The picture's rectangle on a surface, in surface pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PictureRect {
    /// The top-left corner; negative when the surface is smaller than the
    /// picture.
    pub origin: (i32, i32),
    pub size: (u32, u32),
    /// The whole-number vertical scale: each game row is this many rows.
    pub scale: u32,
}

impl PictureRect {
    /// Whether every game pixel is a whole number of surface pixels wide,
    /// so the picture needs no horizontal resampling.
    pub fn crisp(&self) -> bool {
        self.size.0 % WIDTH as u32 == 0
    }
}

/// The picture's width at vertical scale `scale`: height times 4/3 for true
/// 4:3 (rounded), 7/9 of `scale` wide a pixel (rounded) for 4:3 integer,
/// `scale` wide for square pixels.
pub fn picture_width(scale: u32, aspect: Aspect) -> u32 {
    let (w, h) = (WIDTH as u32, HEIGHT as u32);
    match aspect {
        Aspect::True4x3 => (h * scale * 4 + 1) / 3,
        Aspect::Integer4x3 => w * ((scale * 7 + 4) / 9).max(1),
        Aspect::Square => w * scale,
    }
}

/// The picture on a surface of `surface` pixels, centred. The vertical scale
/// is the largest whole number that fits the height and, for true 4:3 and
/// square pixels, the width (at least 1). In 4:3 integer the width steps down
/// a whole number of pixels from the nearest to 7/9 of the scale until it
/// fits.
pub fn picture_rect(surface: (u32, u32), aspect: Aspect) -> PictureRect {
    let tall = (surface.1 / HEIGHT as u32).max(1);
    let (scale, width) = match aspect {
        Aspect::Integer4x3 => {
            let mut wide = (tall * 7 + 4) / 9;
            while wide > 1 && wide * WIDTH as u32 > surface.0 {
                wide -= 1;
            }
            (tall, wide.max(1) * WIDTH as u32)
        }
        _ => {
            let scale = (1..=tall).rev().find(|&s| picture_width(s, aspect) <= surface.0).unwrap_or(1);
            (scale, picture_width(scale, aspect))
        }
    };
    let height = HEIGHT as u32 * scale;
    let centre = |surface: u32, picture: u32| (i64::from(surface) - i64::from(picture)) as i32 / 2;
    PictureRect {
        origin: (centre(surface.0, width), centre(surface.1, height)),
        size: (width, height),
        scale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(surface: (u32, u32), aspect: Aspect) -> ((u32, u32), (i32, i32), u32) {
        let r = picture_rect(surface, aspect);
        (r.size, r.origin, r.scale)
    }

    #[test]
    fn at_3200_by_2000_each_aspect_has_the_ticket_rectangle() {
        assert_eq!(rect((3200, 2000), Aspect::True4x3), ((2389, 1792), (405, 104), 8));
        assert_eq!(rect((3200, 2000), Aspect::Integer4x3), ((2304, 1792), (448, 104), 8));
        assert_eq!(rect((3200, 2000), Aspect::Square), ((3072, 1792), (64, 104), 8));
    }

    #[test]
    fn the_largest_window_scale_fits_the_monitors_work_area() {
        // 3200x2000: 8x is 1792 tall (+120 fits 2000), 9x would be 2016.
        assert_eq!(fit_scale(Some((3200, 2000)), Aspect::True4x3), 8);
        assert_eq!(fit_scale(Some((3200, 2000)), Aspect::Square), 8);
        assert_eq!(fit_scale(Some((1920, 1080)), Aspect::Square), 4);
        assert_eq!(fit_scale(Some((2560, 1440)), Aspect::True4x3), 5);
        // Width can limit it: square pixels are the widest.
        assert_eq!(fit_scale(Some((1000, 4000)), Aspect::Square), 2);
        assert_eq!(fit_scale(Some((1000, 4000)), Aspect::True4x3), 3);
        assert_eq!(fit_scale(Some((200, 100)), Aspect::Square), 1);
        // The ceiling and the unknown monitor.
        assert_eq!(fit_scale(Some((30000, 30000)), Aspect::Square), u32::from(MAX_SCALE));
        assert_eq!(fit_scale(None, Aspect::Square), 4);
        // Every scale up to it fits.
        for scale in 1..=fit_scale(Some((3200, 2000)), Aspect::Square) {
            assert!(picture_width(scale, Aspect::Square) <= 3200 && HEIGHT as u32 * scale <= 2000);
        }
    }

    #[test]
    fn at_1080p_and_1440p() {
        assert_eq!(rect((1920, 1080), Aspect::True4x3), ((1195, 896), (362, 92), 4));
        assert_eq!(rect((1920, 1080), Aspect::Integer4x3), ((1152, 896), (384, 92), 4));
        assert_eq!(rect((1920, 1080), Aspect::Square), ((1536, 896), (192, 92), 4));
        assert_eq!(rect((2560, 1440), Aspect::True4x3), ((1792, 1344), (384, 48), 6));
        assert_eq!(rect((2560, 1440), Aspect::Integer4x3), ((1920, 1344), (320, 48), 6));
        assert_eq!(rect((2560, 1440), Aspect::Square), ((2304, 1344), (128, 48), 6));
    }

    #[test]
    fn the_windows_are_their_own_picture() {
        // A window at scale s is the picture's size, so the picture fills it.
        for aspect in Aspect::ALL {
            for scale in 1..=4 {
                let size = (picture_width(scale, aspect), HEIGHT as u32 * scale);
                assert_eq!(rect(size, aspect), (size, (0, 0), scale), "{aspect:?} at {scale}x");
            }
        }
        assert_eq!(picture_width(3, Aspect::True4x3), 896);
        assert_eq!(picture_width(3, Aspect::Integer4x3), 768);
        assert_eq!(picture_width(3, Aspect::Square), 1152);
    }

    #[test]
    fn small_windows_keep_the_picture_whole_where_they_can() {
        // Wide and short: the height sets the scale.
        assert_eq!(rect((1600, 450), Aspect::True4x3), ((597, 448), (501, 1), 2));
        // Narrow and tall: true 4:3 and square give a scale back for width,
        // 4:3 integer keeps the height's scale and narrows its pixels.
        assert_eq!(rect((1000, 1000), Aspect::True4x3), ((896, 672), (52, 164), 3));
        assert_eq!(rect((1000, 1000), Aspect::Square), ((768, 448), (116, 276), 2));
        assert_eq!(rect((1000, 1000), Aspect::Integer4x3), ((768, 896), (116, 52), 4));
        // Smaller than the game frame: scale 1, hanging over the edges.
        assert_eq!(rect((300, 200), Aspect::Square), ((384, 224), (-42, -12), 1));
        assert_eq!(rect((300, 200), Aspect::Integer4x3), ((384, 224), (-42, -12), 1));
        assert_eq!(rect((300, 200), Aspect::True4x3), ((299, 224), (0, -12), 1));
        // Nothing at all: still a scale of 1, not a divide by zero.
        assert_eq!(rect((0, 0), Aspect::True4x3).2, 1);
    }

    #[test]
    fn always_centred_with_a_whole_vertical_scale() {
        for aspect in Aspect::ALL {
            for surface in [(3200, 2000), (1920, 1080), (2560, 1440), (3840, 2160), (1366, 768), (1280, 800), (800, 600), (640, 480)] {
                let r = picture_rect(surface, aspect);
                assert_eq!(r.size.1, HEIGHT as u32 * r.scale, "{aspect:?} {surface:?}: whole rows");
                assert!(r.size.0 <= surface.0 && r.size.1 <= surface.1, "{aspect:?} {surface:?}: fits");
                let (left, top) = (r.origin.0 as u32, r.origin.1 as u32);
                assert!(surface.0 - r.size.0 - left - left <= 1, "{aspect:?} {surface:?}: centred across");
                assert!(surface.1 - r.size.1 - top - top <= 1, "{aspect:?} {surface:?}: centred down");
                if aspect != Aspect::True4x3 {
                    assert!(r.crisp(), "{aspect:?} {surface:?}: whole-number pixels");
                }
            }
        }
    }
}
