use crate::compositor::IndexedSprite;

use super::accent::AccentKind;

const GOLD: [u32; 16] = [
    0, 0xff6e3622, 0xffab612d, 0xffe39442, 0xffffce72, 0xfffff0bb, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
const FIRE: [u32; 16] = [
    0, 0xff852225, 0xffc73525, 0xfff46728, 0xffffb83b, 0xffffed9d, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
const SPIRAL_FIRE: [u32; 16] = [
    0, 0xff852225, 0xffc73525, 0xfff46728, 0xffffbc4a, 0xfffff2b3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
const COLUMN_FIRE: [u32; 16] = [
    0, 0xff852225, 0xffc73525, 0xfff46728, 0xffffbf54, 0xffffe9a8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
const GUREN_FIRE: [u32; 16] = [
    0, 0xff822234, 0xffbd2930, 0xffee4c26, 0xffff9638, 0xffffdf84, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

// Each path follows the visible heel, with the last value giving the local
// radius. A zero-radius end breaks into a single pixel.
type Path = &'static [(i32, i32, i32)];
const JAGUAR: Path = &[
    (-27, -7, 0),
    (-19, -6, 0),
    (-10, -3, 1),
    (-2, 1, 2),
    (4, 3, 2),
    (9, 1, 1),
    (11, -3, 0),
];
const JAGUAR_LOW: Path = &[
    (-31, 2, 0),
    (-22, 5, 0),
    (-13, 8, 1),
    (-3, 9, 2),
    (5, 7, 2),
    (13, 2, 1),
    (15, -4, 0),
];
const JAGUAR_RISE: Path = &[
    (-22, 27, 0),
    (-12, 23, 0),
    (-3, 16, 1),
    (5, 7, 3),
    (7, -2, 2),
    (4, -8, 1),
    (-1, -13, 0),
];
const HEEL_FIRE: Path = &[
    (-32, 3, 0),
    (-24, 1, 1),
    (-19, 3, 1),
    (-12, 2, 3),
    (-5, -1, 4),
    (1, -3, 4),
    (8, -5, 2),
    (12, -8, 0),
];
const HEEL_FLARE: Path = &[
    (-24, -8, 0),
    (-18, -7, 1),
    (-13, -4, 2),
    (-7, -3, 2),
    (1, -3, 1),
];
const DRAGONLASH: Path = &[
    (-30, 11, 0),
    (-22, 8, 1),
    (-13, 6, 2),
    (-4, 1, 4),
    (3, -4, 5),
    (7, -9, 3),
    (8, -14, 2),
    (5, -19, 0),
];
const SPIRAL_BACK: Path = &[
    (-29, 11, 0),
    (-32, 7, 1),
    (-27, 2, 2),
    (-15, -1, 2),
    (0, -3, 2),
    (12, -5, 1),
    (20, -10, 0),
];
const SPIRAL_FRONT: Path = &[
    (-32, 18, 0),
    (-21, 21, 1),
    (-7, 20, 2),
    (11, 15, 4),
    (24, 9, 4),
    (30, 2, 3),
    (29, -5, 2),
    (22, -11, 1),
    (13, -14, 0),
];

pub(super) fn kick_accent(
    kind: AccentKind,
    age: u8,
    x: i32,
    y: i32,
    flip: bool,
) -> Vec<IndexedSprite> {
    use AccentKind::*;

    let phase = i32::from(age / 2 % 3);
    let mut canvas = Canvas::new(flip);
    let palette = match kind {
        JaguarKick => {
            canvas.ribbon(JAGUAR, 0, 0, 5);
            canvas.embers(&[(-21, -10), (-15, 6), (9, -5)], phase, 3);
            GOLD
        }
        JaguarRevolver => {
            canvas.ribbon(JAGUAR, 0, -4, 4);
            canvas.ribbon(JAGUAR_LOW, 0, 0, 5);
            canvas.embers(&[(-27, -5), (-21, 9), (11, -5)], phase, 4);
            GOLD
        }
        JaguarRise => {
            canvas.ribbon(JAGUAR_RISE, -5, -2, 3);
            canvas.ribbon(JAGUAR_RISE, 0, 0, 5);
            canvas.embers(&[(-14, 18), (-3, 24), (9, -12)], phase, 4);
            GOLD
        }
        ShippuKick => {
            canvas.ribbon(HEEL_FLARE, 0, -phase, 4);
            canvas.ribbon(HEEL_FIRE, 0, 0, 5);
            canvas.embers(&[(-27, -5), (-20, 6), (-12, -10), (8, 1)], phase, 4);
            FIRE
        }
        ShippuSpiral => {
            canvas.ribbon(SPIRAL_BACK, 2, -phase, 3);
            canvas.ribbon(SPIRAL_FRONT, 0, -phase, 5);
            canvas.ribbon(HEEL_FLARE, 12, 10 - phase, 4);
            canvas.embers(
                &[(-28, 14), (-20, -1), (25, -11), (31, 7), (12, 18)],
                phase,
                4,
            );
            SPIRAL_FIRE
        }
        DragonlashFlame => {
            canvas.ribbon(HEEL_FLARE, 0, -phase, 4);
            canvas.ribbon(DRAGONLASH, 0, 0, 5);
            canvas.embers(&[(-29, 14), (-19, 10), (10, -13), (-4, -9)], phase, 4);
            FIRE
        }
        GurenSweep => {
            canvas.ribbon(
                &[
                    (-34, -6, 0),
                    (-23, -13, 1),
                    (-8, -15, 3),
                    (9, -12, 3),
                    (25, -8, 2),
                    (34, -3, 0),
                ],
                0,
                -phase,
                4,
            );
            canvas.ribbon(
                &[
                    (-33, 5, 0),
                    (-23, 10, 2),
                    (-5, 12, 4),
                    (15, 9, 5),
                    (28, 4, 5),
                    (34, -2, 2),
                    (33, -7, 0),
                ],
                0,
                0,
                5,
            );
            canvas.ribbon(HEEL_FLARE, 8, 11, 3);
            canvas.embers(
                &[(-28, 9), (-14, -17), (8, 16), (28, 11), (34, -10)],
                phase,
                4,
            );
            GUREN_FIRE
        }
        Shinryuken => {
            canvas.ribbon(
                &[
                    (-11, 37, 0),
                    (-19, 28, 1),
                    (-16, 13, 3),
                    (-7, 1, 2),
                    (-5, -14, 3),
                    (3, -27, 2),
                    (9, -43, 0),
                ],
                -phase,
                0,
                4,
            );
            canvas.ribbon(
                &[
                    (10, 38, 0),
                    (20, 31, 2),
                    (20, 22, 3),
                    (10, 15, 5),
                    (-7, 10, 4),
                    (-19, 4, 3),
                    (-21, -4, 2),
                    (-15, -10, 0),
                ],
                0,
                phase,
                5,
            );
            canvas.ribbon(
                &[
                    (-18, -13, 0),
                    (-8, -17, 2),
                    (8, -17, 3),
                    (18, -22, 3),
                    (17, -29, 2),
                    (8, -36, 1),
                    (6, -43, 0),
                ],
                phase,
                -phase,
                5,
            );
            canvas.embers(
                &[
                    (-18, 35),
                    (-24, 23),
                    (23, 16),
                    (-24, -7),
                    (22, -27),
                    (16, -38),
                ],
                phase,
                4,
            );
            COLUMN_FIRE
        }
        _ => return Vec::new(),
    };
    canvas.into_sprites(x, y, palette)
}

const WIDTH: usize = 80;
const HEIGHT: usize = 96;

struct Canvas {
    pens: [[u8; WIDTH]; HEIGHT],
    flip: bool,
}

impl Canvas {
    fn new(flip: bool) -> Self {
        Self {
            pens: [[0; WIDTH]; HEIGHT],
            flip,
        }
    }

    fn plot(&mut self, x: i32, y: i32, pen: u8) {
        let x = WIDTH as i32 / 2 + if self.flip { -x } else { x };
        let y = HEIGHT as i32 / 2 + y;
        if let Some(cell) = self
            .pens
            .get_mut(y as usize)
            .and_then(|row| row.get_mut(x as usize))
        {
            *cell = (*cell).max(pen);
        }
    }

    fn ribbon(&mut self, path: Path, dx: i32, dy: i32, pen: u8) {
        for segment in path.windows(2) {
            let (x0, y0, r0) = segment[0];
            let (x1, y1, r1) = segment[1];
            let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
            for step in 0..=steps {
                let x = dx + x0 + (x1 - x0) * step / steps;
                let y = dy + y0 + (y1 - y0) * step / steps;
                let radius = (r0 * (steps - step) + r1 * step + steps / 2) / steps;
                for sy in -radius..=radius {
                    for sx in -radius..=radius {
                        let distance = sx.abs() + sy.abs();
                        if distance <= radius {
                            let shade = if radius == 0 {
                                1
                            } else {
                                pen.saturating_sub(distance as u8).max(1)
                            };
                            self.plot(x + sx, y + sy, shade);
                        }
                    }
                }
            }
        }
    }

    fn embers(&mut self, points: &[(i32, i32)], phase: i32, pen: u8) {
        for (index, &(x, y)) in points.iter().enumerate() {
            self.plot(
                x - phase,
                y - phase * (1 + (index % 2) as i32),
                pen - phase as u8,
            );
        }
    }

    fn into_sprites(self, x: i32, y: i32, palette: [u32; 16]) -> Vec<IndexedSprite> {
        let mut sprites = Vec::new();
        for top in (0..HEIGHT).step_by(16) {
            for left in (0..WIDTH).step_by(16) {
                let pixels = std::array::from_fn(|i| self.pens[top + i / 16][left + i % 16]);
                if pixels.iter().any(|&pen| pen != 0) {
                    sprites.push(IndexedSprite {
                        x: x - WIDTH as i32 / 2 + left as i32,
                        y: y - HEIGHT as i32 / 2 + top as i32,
                        pixels,
                        palette,
                    });
                }
            }
        }
        sprites
    }
}
