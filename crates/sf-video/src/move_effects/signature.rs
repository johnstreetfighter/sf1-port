//! Original native-pixel effects for the Denjin, Tiger Cannon and Zan'ei preview.

use std::sync::LazyLock;

use super::accent::{accent_frame, AccentKind};
use super::power::{power_impact, power_projectile};
use super::{decode_sheet, SIZE};
use crate::compositor::IndexedSprite;

static DENJIN_HANDS: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("denjin-hands.png"), 4, (16, 8)));
static DENJIN_PROJECTILE: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("denjin-projectile.png"), 4, (16, 16)));
static DENJIN_IMPACT: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("denjin-impact.png"), 6, (16, 16)));
static TIGER_CANNON: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("tiger-cannon.png"), 4, (28, 16)));
static TIGER_IMPACT: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("tiger-impact.png"), 6, (24, 8)));
static ZANEI_IMPACT: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("zanei-impact.png"), 5, (16, 16)));
static LIGHT_HIT: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("hit-light.png"), 3, (8, 8)));
static MEDIUM_HIT: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("hit-medium.png"), 4, (15, 15)));
static HEAVY_HIT: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("hit-heavy.png"), 6, (15, 15)));
static DRIVE_HIT: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("drive-impact-hit.png"), 6, (28, 24)));
static DRIVE_GUARD: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("drive-impact-guard.png"), 5, (24, 23)));
static SLAM_DUST: LazyLock<Strip> =
    LazyLock::new(|| Strip::decode(include_bytes!("slam-dust.png"), 7, (39, 24)));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileKind {
    Denjin,
    TigerCannon,
    GekiShuriken,
    Shinku,
    Metsu,
    Shakunetsu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImpactKind {
    Denjin,
    TigerCannon,
    Zanei,
    GekiShuriken,
    Shinku,
    Metsu,
    Shakunetsu,
    Satsui,
    LightHit,
    MediumHit,
    HeavyHit,
    DriveImpactHit,
    DriveImpactGuard,
    SlamDust,
}

impl ImpactKind {
    /// Length of the one-shot effect in 60 Hz game ticks.
    pub const fn lifetime(self) -> u8 {
        match self {
            Self::Denjin | Self::TigerCannon => 12,
            Self::Zanei => 5,
            Self::GekiShuriken | Self::Satsui => 8,
            Self::Shinku | Self::Metsu | Self::Shakunetsu => 10,
            Self::LightHit => 6,
            Self::MediumHit => 8,
            Self::HeavyHit | Self::DriveImpactHit => 12,
            Self::DriveImpactGuard => 10,
            Self::SlamDust => 14,
        }
    }
}

/// Electrical arcs centered between Ryu's hands, repeating while charge shows.
pub fn denjin_hands(age: u8, x: i32, y: i32, flip: bool) -> Vec<IndexedSprite> {
    DENJIN_HANDS.place(usize::from(age / 2) % 4, x, y, flip)
}

/// Accent placed over the center of the moving native projectile.
pub fn signature_projectile(
    kind: ProjectileKind,
    age: u8,
    x: i32,
    y: i32,
    flip: bool,
) -> Vec<IndexedSprite> {
    let (strip, hold) = match kind {
        ProjectileKind::Denjin => (&*DENJIN_PROJECTILE, 2),
        ProjectileKind::TigerCannon => (&*TIGER_CANNON, 3),
        ProjectileKind::GekiShuriken => {
            return accent_frame(AccentKind::GekiShuriken, age, x, y, flip)
        }
        ProjectileKind::Shinku | ProjectileKind::Metsu | ProjectileKind::Shakunetsu => {
            return power_projectile(kind, age, x, y, flip);
        }
    };
    strip.place(usize::from(age / hold) % 4, x, y, flip)
}

/// One-shot flash placed at a contact or the release of a charged palm burst.
pub fn signature_impact(
    kind: ImpactKind,
    age: u8,
    x: i32,
    y: i32,
    flip: bool,
) -> Vec<IndexedSprite> {
    if age >= kind.lifetime() {
        return Vec::new();
    }
    let (strip, hold) = match kind {
        ImpactKind::Denjin => (&*DENJIN_IMPACT, 2),
        ImpactKind::TigerCannon => (&*TIGER_IMPACT, 2),
        ImpactKind::Zanei => (&*ZANEI_IMPACT, 1),
        ImpactKind::GekiShuriken => {
            return accent_frame(AccentKind::GekiShurikenImpact, age, x, y, flip)
        }
        ImpactKind::Satsui => return accent_frame(AccentKind::SatsuiImpact, age, x, y, flip),
        ImpactKind::LightHit => (&*LIGHT_HIT, 2),
        ImpactKind::MediumHit => (&*MEDIUM_HIT, 2),
        ImpactKind::HeavyHit => (&*HEAVY_HIT, 2),
        ImpactKind::DriveImpactHit => (&*DRIVE_HIT, 2),
        ImpactKind::DriveImpactGuard => (&*DRIVE_GUARD, 2),
        ImpactKind::SlamDust => (&*SLAM_DUST, 2),
        ImpactKind::Shinku | ImpactKind::Metsu | ImpactKind::Shakunetsu => {
            return power_impact(kind, age, x, y, flip);
        }
    };
    strip.place(usize::from(age / hold), x, y, flip)
}

pub(super) struct Strip {
    /// Tile offsets are stored relative to the frame's animation anchor.
    frames: Vec<Vec<IndexedSprite>>,
}

impl Strip {
    pub(super) fn decode(bytes: &[u8], frame_count: usize, anchor: (i32, i32)) -> Self {
        let (tiles, width, height) = decode_sheet(bytes);
        let stride = width / SIZE;
        assert_eq!(stride % frame_count, 0);
        let frame_cols = stride / frame_count;
        let frames = (0..frame_count)
            .map(|frame| {
                let mut sprites = Vec::new();
                for ty in 0..height / SIZE {
                    for tx in 0..frame_cols {
                        let tile = &tiles[ty * stride + frame * frame_cols + tx];
                        if tile.pixels.iter().any(|&pen| pen != 0) {
                            sprites.push(IndexedSprite {
                                x: (tx * SIZE) as i32 - anchor.0,
                                y: (ty * SIZE) as i32 - anchor.1,
                                pixels: tile.pixels,
                                palette: tile.palette.map(|color| {
                                    u32::from_be_bytes([color.a, color.r, color.g, color.b])
                                }),
                            });
                        }
                    }
                }
                sprites
            })
            .collect();
        Self { frames }
    }

    pub(super) fn place(&self, frame: usize, x: i32, y: i32, flip: bool) -> Vec<IndexedSprite> {
        let mut sprites = self.frames[frame].clone();
        for sprite in &mut sprites {
            if flip {
                sprite.x = -sprite.x - (SIZE as i32 - 1);
                for row in sprite.pixels.chunks_exact_mut(SIZE) {
                    row.reverse();
                }
            }
            sprite.x += x;
            sprite.y += y;
        }
        sprites
    }
}
