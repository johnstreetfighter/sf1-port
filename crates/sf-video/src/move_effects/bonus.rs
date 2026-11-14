//! Original pixel accents for Geki and Evil Ryu.

use std::sync::LazyLock;

use super::{accent::AccentKind, signature::Strip};
use crate::compositor::IndexedSprite;

static SHURIKEN: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("geki-shuriken.png"), 4, (23, 8)));
static SHURIKEN_HIT: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("geki-shuriken-hit.png"), 4, (16, 16)));
static VANISH: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("geki-vanish.png"), 4, (24, 24)));
static SATSUI_GLIDE: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("satsui-glide.png"), 4, (34, 20)));
static SATSUI_HIT: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("satsui-hit.png"), 4, (24, 16)));

pub(super) fn bonus_accent(
    kind: AccentKind,
    age: u8,
    x: i32,
    y: i32,
    flip: bool,
) -> Vec<IndexedSprite> {
    let frame = usize::from(age / 2);
    let (strip, frame) = match kind {
        AccentKind::GekiShuriken => (&*SHURIKEN, frame % 4),
        AccentKind::GekiVanish => (&*VANISH, usize::from(age / 3) % 4),
        AccentKind::SatsuiGlide => (&*SATSUI_GLIDE, frame % 4),
        AccentKind::GekiShurikenImpact if frame < 4 => (&*SHURIKEN_HIT, frame),
        AccentKind::SatsuiImpact if frame < 4 => (&*SATSUI_HIT, frame),
        _ => return Vec::new(),
    };
    strip.place(frame, x, y, flip)
}
