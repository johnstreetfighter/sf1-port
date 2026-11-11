//! 1987 bonus challengers (`#117` M4, `#118` M5, `#119` M6).
//!
//! They appear on the select screen only when bonus challengers are enabled
//! and the filter is showing the expanded roster. Each one has an SF1 kit
//! with the 1987 player rules, later era styles, a stage, and an FM theme.

use crate::era::EraFamily;
use crate::roster::TournamentFighter;
use sf_video::asset_pack::{
    encode_adpcm, quantize_sheet, simple_fm_patch, validate_adpcm, validate_fm_patch, validate_tile,
    AdpcmClip, FmPatch, IndexedTile, Rgba,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Challenger {
    Gouken,
    Akuma,
    EvilRyu,
    Goutetsu,
    ShengLong,
    GouHibiki,
    Dan,
    Cody,
    Guy,
    Haggar,
    Nash,
    Bison,
    ChunLi,
}

impl Challenger {
    pub const ALL: [Challenger; 13] = [
        Challenger::Gouken,
        Challenger::Akuma,
        Challenger::EvilRyu,
        Challenger::Goutetsu,
        Challenger::ShengLong,
        Challenger::GouHibiki,
        Challenger::Dan,
        Challenger::Cody,
        Challenger::Guy,
        Challenger::Haggar,
        Challenger::Nash,
        Challenger::Bison,
        Challenger::ChunLi,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Challenger::Gouken => "Gouken",
            Challenger::Akuma => "Akuma",
            Challenger::EvilRyu => "Evil Ryu",
            Challenger::Goutetsu => "Goutetsu",
            Challenger::ShengLong => "Sheng Long",
            Challenger::GouHibiki => "Gou Hibiki",
            Challenger::Dan => "Dan",
            Challenger::Cody => "Cody",
            Challenger::Guy => "Guy",
            Challenger::Haggar => "Haggar",
            Challenger::Nash => "Charlie",
            Challenger::Bison => "M. Bison",
            Challenger::ChunLi => "Chun-Li",
        }
    }

    pub const fn specials(self) -> &'static [&'static str] {
        match self {
            Challenger::Gouken => &["Gohadoken", "Senkugoshoha", "Tatsumaki Gorasen", "Forbidden Shoryuken"],
            Challenger::Akuma => &["Gohadoken", "Zanku Hadoken", "Goshoryuken", "Hyakkishu", "Shun Goku Satsu"],
            Challenger::EvilRyu => &["Hadoken", "Shakunetsu", "Ashura Senkuu", "Metsu Shoryuken", "Shun Goku Satsu"],
            Challenger::Goutetsu => &["Ansatsuken Strike", "Teleport Step", "Kiai", "Counter Throw"],
            Challenger::ShengLong => &["Flying Dragon Fist", "Rising Dragon Kick", "Teleport"],
            Challenger::GouHibiki => &["Gadoken", "Koryuken", "Haoh Gadoken", "Proto Counter"],
            Challenger::Dan => &["Gadoken", "Koryuken", "Dankukyaku", "Taunt", "Hissho Buraiken", "Legendary Taunt"],
            Challenger::Cody => &["Bad Stone", "Brawler Combo", "Criminal Upper", "Ruffian Kick", "Final Destruction"],
            Challenger::Guy => &["Bushin Senpukyaku", "Hayagake", "Hozanto", "Bushin Izuna Otoshi"],
            Challenger::Haggar => &["Double Lariat", "Violent Axe", "Flying Piledriver", "Spinning Piledriver", "Giant Haggar Press"],
            Challenger::Nash => &["Sonic Boom", "Somersault Justice", "Moonsault Slash", "Crossfire Blitz"],
            Challenger::Bison => &["Psycho Crusher", "Double Knee Press", "Head Press", "Somersault Skull Diver", "Psycho Shot"],
            Challenger::ChunLi => &["Hyakuretsukyaku", "Kikoken", "Spinning Bird Kick", "Senretsukyaku"],
        }
    }

    pub const fn stage(self) -> &'static str {
        match self {
            Challenger::Gouken | Challenger::Akuma | Challenger::EvilRyu => "Japan Temple Night",
            Challenger::Goutetsu | Challenger::ShengLong => "Dojo",
            Challenger::GouHibiki | Challenger::Dan => "Thailand Shrine",
            Challenger::Cody | Challenger::Guy => "Metro City Alley",
            Challenger::Haggar => "Wrestling Ring",
            Challenger::Nash => "US Harbor",
            Challenger::Bison => "Shadaloo Temple",
            Challenger::ChunLi => "China Courtyard",
        }
    }

    pub const fn styles(self) -> [EraFamily; 8] {
        let _ = self;
        [
            EraFamily::Sf1987,
            EraFamily::Sf1Plus,
            EraFamily::Sf2,
            EraFamily::SfZ,
            EraFamily::Sf3,
            EraFamily::Sf4,
            EraFamily::Sf5,
            EraFamily::Sf6,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BonusCostume {
    Default,
    Tengu,
    Prison,
    Mayor,
    Stitched,
    Qipao,
    Master,
}

pub fn costumes(fighter: Challenger) -> &'static [BonusCostume] {
    match fighter {
        Challenger::GouHibiki => &[BonusCostume::Default, BonusCostume::Tengu],
        Challenger::Cody => &[BonusCostume::Default, BonusCostume::Prison, BonusCostume::Mayor],
        Challenger::Nash => &[BonusCostume::Default, BonusCostume::Stitched],
        Challenger::ChunLi => &[BonusCostume::Default, BonusCostume::Qipao, BonusCostume::Master],
        _ => &[BonusCostume::Default],
    }
}

pub fn roster_row() -> String {
    let names = Challenger::ALL.iter().map(|one| one.name()).collect::<Vec<_>>().join("  ");
    format!("[BONUS '87]  {names}")
}

#[derive(Debug, Clone)]
pub struct BonusFighter {
    pub fighter: Challenger,
    pub family: EraFamily,
    pub costume: BonusCostume,
    threw: bool,
    pub tiles: Vec<IndexedTile>,
    pub voice: AdpcmClip,
    pub patch: FmPatch,
    /// The file name of the challenger's own track, as the Jukebox takes it.
    pub theme: String,
}

impl BonusFighter {
    pub fn new(fighter: Challenger, family: EraFamily, costume: BonusCostume) -> Self {
        let tiles = tiles_for(fighter);
        for tile in &tiles {
            validate_tile(tile).expect("bonus tile");
        }
        let voice = encode_adpcm(&cue(fighter)).expect("bonus voice");
        validate_adpcm(&voice).expect("bonus voice");
        let patch = simple_fm_patch(index(fighter), 0x12);
        validate_fm_patch(&patch).expect("bonus patch");
        Self {
            fighter,
            family,
            costume,
            threw: false,
            tiles,
            voice,
            patch,
            theme: format!("cd/{}.wav", slug(fighter)),
        }
    }

    pub fn special(&self, name: &str) -> bool {
        self.fighter.specials().contains(&name)
    }

    pub fn intro(&self) -> &'static str {
        if self.fighter == Challenger::Bison { "Cape" } else { "Ready" }
    }

    /// SF1 uses the player hitstun, crouch, jump, and block. No boss lock.
    pub fn sf1_rules(&self) -> Option<Sf1PlayerRules> {
        if self.family != EraFamily::Sf1987 {
            return None;
        }
        Some(Sf1PlayerRules { hitstun: 26, crouch: true, jump: true, block: true, boss_lock: false })
    }

    pub fn command_throw(&mut self) -> bool {
        if self.fighter != Challenger::Haggar || self.threw {
            return false;
        }
        self.threw = true;
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sf1PlayerRules {
    pub hitstun: u8,
    pub crouch: bool,
    pub jump: bool,
    pub block: bool,
    pub boss_lock: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Side {
    Tourney(TournamentFighter),
    Bonus(Challenger),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bout {
    pub left: Side,
    pub right: Side,
    pub left_family: EraFamily,
    pub right_family: EraFamily,
    hp_left: u16,
    hp_right: u16,
    frame: u32,
}

impl Bout {
    pub fn new(left: Side, left_family: EraFamily, right: Side, right_family: EraFamily) -> Self {
        Self { left, right, left_family, right_family, hp_left: 144, hp_right: 144, frame: 0 }
    }

    pub fn step(&mut self, damage: u16) {
        self.frame += 1;
        self.hp_right = self.hp_right.saturating_sub(damage);
    }

    pub fn snapshot(&self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[0..2].copy_from_slice(&self.hp_left.to_le_bytes());
        out[2..4].copy_from_slice(&self.hp_right.to_le_bytes());
        out[4..8].copy_from_slice(&self.frame.to_le_bytes());
        out[8] = self.left_family as u8;
        out[9] = self.right_family as u8;
        out[10] = side_tag(&self.left);
        out[11] = side_tag(&self.right);
        out
    }

    pub fn restore(&mut self, snap: &[u8; 16]) {
        self.hp_left = u16::from_le_bytes([snap[0], snap[1]]);
        self.hp_right = u16::from_le_bytes([snap[2], snap[3]]);
        self.frame = u32::from_le_bytes([snap[4], snap[5], snap[6], snap[7]]);
    }
}

fn side_tag(side: &Side) -> u8 {
    match side {
        Side::Tourney(fighter) => *fighter as u8,
        Side::Bonus(fighter) => 0x80 | index(*fighter),
    }
}

fn index(fighter: Challenger) -> u8 {
    Challenger::ALL.iter().position(|one| *one == fighter).unwrap_or(0) as u8
}

fn slug(fighter: Challenger) -> &'static str {
    match fighter {
        Challenger::Gouken => "gouken",
        Challenger::Akuma => "akuma",
        Challenger::EvilRyu => "evil-ryu",
        Challenger::Goutetsu => "goutetsu",
        Challenger::ShengLong => "sheng-long",
        Challenger::GouHibiki => "gou-hibiki",
        Challenger::Dan => "dan",
        Challenger::Cody => "cody",
        Challenger::Guy => "guy",
        Challenger::Haggar => "haggar",
        Challenger::Nash => "nash",
        Challenger::Bison => "bison",
        Challenger::ChunLi => "chun-li",
    }
}

fn cue(fighter: Challenger) -> Vec<i16> {
    let base = 40 + i16::from(index(fighter)) * 11;
    (0..32).map(|i| base + i * 7).collect()
}

fn tiles_for(fighter: Challenger) -> Vec<IndexedTile> {
    let mut pixels = vec![Rgba::TRANSPARENT; 256];
    let ink = index(fighter).wrapping_mul(12);
    for y in 1..15 {
        for x in 2..14 {
            if (x + y + index(fighter) as usize) % 2 == 0 {
                pixels[y * 16 + x] = Rgba { r: 80 + ink, g: 36, b: 24, a: 255 };
            }
        }
    }
    quantize_sheet(&pixels, 16, 16).expect("bonus sheet")
}
