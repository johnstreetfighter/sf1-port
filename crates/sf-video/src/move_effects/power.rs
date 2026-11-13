use std::sync::LazyLock;

use super::accent::AccentKind;
use super::signature::{ImpactKind, ProjectileKind, Strip};
use crate::compositor::IndexedSprite;

static SHINKU: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("shinku-flight.png"), 4, (24, 16)));
static METSU: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("metsu-flight.png"), 4, (24, 16)));
static SHAKUNETSU: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("shakunetsu-flight.png"), 4, (24, 16)));
static SHINKU_IMPACT: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("shinku-impact.png"), 5, (23, 23)));
static METSU_IMPACT: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("metsu-impact.png"), 5, (23, 23)));
static SHAKUNETSU_IMPACT: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("shakunetsu-impact.png"), 5, (23, 23)));
static SHIN_RISE: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("shin-rise.png"), 4, (29, 12)));
static METSU_RISE: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("metsu-rise.png"), 4, (29, 12)));
static TIGER_KNEE: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("tiger-knee.png"), 4, (28, 13)));
static TIGER_RISE: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("tiger-rise.png"), 4, (29, 12)));
static DRIVE_IMPACT: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("drive-impact.png"), 4, (28, 22)));
static DRIVE_SWING: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("drive-impact-swing.png"), 3, (34, 20)));
static DRIVE_RUSH: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("drive-rush.png"), 4, (38, 18)));
static FOCUS_INK: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("focus-ink.png"), 4, (31, 23)));

pub(super) fn power_projectile(
    kind: ProjectileKind,
    age: u8,
    x: i32,
    y: i32,
    flip: bool,
) -> Vec<IndexedSprite> {
    let strip = match kind {
        ProjectileKind::Shinku => &*SHINKU,
        ProjectileKind::Metsu => &*METSU,
        ProjectileKind::Shakunetsu => &*SHAKUNETSU,
        _ => return Vec::new(),
    };
    strip.place(usize::from(age / 2) % 4, x, y, flip)
}

pub(super) fn power_impact(
    kind: ImpactKind,
    age: u8,
    x: i32,
    y: i32,
    flip: bool,
) -> Vec<IndexedSprite> {
    let strip = match kind {
        ImpactKind::Shinku => &*SHINKU_IMPACT,
        ImpactKind::Metsu => &*METSU_IMPACT,
        ImpactKind::Shakunetsu => &*SHAKUNETSU_IMPACT,
        _ => return Vec::new(),
    };
    strip.place(usize::from(age / 2), x, y, flip)
}

pub(super) fn power_accent(
    kind: AccentKind,
    age: u8,
    x: i32,
    y: i32,
    flip: bool,
) -> Vec<IndexedSprite> {
    let strip = match kind {
        AccentKind::DriveImpact => {
            let frame = usize::from(age / 2);
            return if frame < 4 {
                DRIVE_IMPACT.place(frame, x, y, flip)
            } else {
                Vec::new()
            };
        }
        AccentKind::DriveImpactSwing => {
            return DRIVE_SWING.place(usize::from(age).min(2), x, y, flip)
        }
        AccentKind::ShinRise => &*SHIN_RISE,
        AccentKind::MetsuRise => &*METSU_RISE,
        AccentKind::TigerKnee => &*TIGER_KNEE,
        AccentKind::TigerRise => &*TIGER_RISE,
        AccentKind::DriveRush => &*DRIVE_RUSH,
        AccentKind::FocusInk => &*FOCUS_INK,
        _ => return Vec::new(),
    };
    strip.place(usize::from(age / 2) % 4, x, y, flip)
}
