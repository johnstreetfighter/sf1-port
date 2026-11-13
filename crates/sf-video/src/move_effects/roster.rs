//! Original four-frame pixel-art strips for the roster's move accents.

use std::sync::LazyLock;

use super::{accent::AccentKind, signature::Strip};
use crate::compositor::IndexedSprite;

const FRAMES: usize = 4;

static BIRDIE_RUSH: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("roster/birdie-rush.png"), FRAMES, (27, 12)));
static BIRDIE_SUPER: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("roster/birdie-super.png"), FRAMES, (43, 12)));
static EAGLE_STRIKE: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("roster/eagle-strike.png"), FRAMES, (19, 16)));
static EAGLE_SUPER: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("roster/eagle-super.png"), FRAMES, (19, 16)));
static RETSU_KICK: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("roster/retsu-kick.png"), FRAMES, (22, 6)));
static RETSU_KIAI: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("roster/retsu-kiai.png"), FRAMES, (16, 16)));
static LEE_PALM: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("roster/lee-palm.png"), FRAMES, (17, 18)));
static MIKE_PUNCH: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("roster/mike-punch.png"), FRAMES, (23, 15)));
static JOE_KICK: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("roster/joe-kick.png"), FRAMES, (22, 15)));

pub(super) fn roster_accent(
    kind: AccentKind,
    age: u8,
    x: i32,
    y: i32,
    flip: bool,
) -> Vec<IndexedSprite> {
    use AccentKind::*;

    let strip = match kind {
        BirdieRush => &BIRDIE_RUSH,
        BirdieSuper => &BIRDIE_SUPER,
        EagleStrike => &EAGLE_STRIKE,
        EagleSuper => &EAGLE_SUPER,
        RetsuKick => &RETSU_KICK,
        RetsuKiai if usize::from(age) < FRAMES * 2 => &RETSU_KIAI,
        LeePalm => &LEE_PALM,
        MikePunch => &MIKE_PUNCH,
        JoeKick => &JOE_KICK,
        _ => return Vec::new(),
    };
    let frame = (usize::from(age) / 2) % FRAMES;
    strip.place(frame, x, y, flip)
}
