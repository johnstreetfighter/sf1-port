//! Fighter-attached effects for the Overhaul preview.

use crate::compositor::IndexedSprite;

use super::{bonus::bonus_accent, kicks::kick_accent, power::power_accent, roster::roster_accent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccentKind {
    JaguarKick,
    JaguarRevolver,
    JaguarRise,
    ShippuKick,
    ShippuSpiral,
    DragonlashFlame,
    Shinryuken,
    GurenSweep,
    BirdieRush,
    BirdieSuper,
    EagleStrike,
    EagleSuper,
    RetsuKick,
    RetsuKiai,
    LeePalm,
    MikePunch,
    JoeKick,
    GekiShuriken,
    GekiShurikenImpact,
    GekiVanish,
    SatsuiGlide,
    SatsuiImpact,
    ShinRise,
    MetsuRise,
    TigerKnee,
    TigerRise,
    DriveImpact,
    DriveImpactSwing,
    DriveRush,
    FocusInk,
}

pub fn accent_frame(kind: AccentKind, age: u8, x: i32, y: i32, flip: bool) -> Vec<IndexedSprite> {
    use AccentKind::*;

    match kind {
        JaguarKick | JaguarRevolver | JaguarRise | ShippuKick | ShippuSpiral | DragonlashFlame
        | Shinryuken | GurenSweep => kick_accent(kind, age, x, y, flip),
        BirdieRush | BirdieSuper | EagleStrike | EagleSuper | RetsuKick | RetsuKiai | LeePalm
        | MikePunch | JoeKick => roster_accent(kind, age, x, y, flip),
        GekiShuriken | GekiShurikenImpact | GekiVanish | SatsuiGlide | SatsuiImpact => {
            bonus_accent(kind, age, x, y, flip)
        }
        ShinRise | MetsuRise | TigerKnee | TigerRise | DriveImpact | DriveImpactSwing
        | DriveRush | FocusInk => power_accent(kind, age, x, y, flip),
    }
}
