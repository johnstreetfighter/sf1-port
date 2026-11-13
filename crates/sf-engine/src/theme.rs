//! Character theme identities and stage associations (ADR 0023).

// The serialized slug and driver command have independent meanings. Neither
// relies on roster order or on the name of a score file.
macro_rules! theme_registry {
    ($($id:ident, $slug:literal, $command:expr;)*) => {
        /// A stable theme identity, including the original test composition.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum ThemeId {
            $( $id, )*
        }

        impl ThemeId {
            pub const ALL: [Self; 41] = [$(Self::$id,)*];

            pub const fn slug(self) -> &'static str {
                match self { $(Self::$id => $slug,)* }
            }

            pub fn from_slug(slug: &str) -> Option<Self> {
                match slug { $($slug => Some(Self::$id),)* _ => None }
            }

            pub const fn command(self) -> u8 {
                match self { $(Self::$id => $command,)* }
            }

            pub const fn from_command(command: u8) -> Option<Self> {
                match command { $(value if value == $command => Some(Self::$id),)* _ => None }
            }
        }
    };
}

theme_registry! {
    Retsu, "retsu", 0x28;
    Geki, "geki", 0x29;
    Joe, "joe", 0x2a;
    Mike, "mike", 0x2b;
    Birdie, "birdie", 0x2c;
    Eagle, "eagle", 0x2d;
    Lee, "lee", 0x2e;
    Gen, "gen", 0x2f;
    Adon, "adon", 0x30;
    Sagat, "sagat", 0x31;
    Test, "test", 0x40;
    Ryu, "ryu", sf_video::background_bank::RYU_THEME;
    Bison, "bison", 0x42;
    Gouken, "gouken", 0x43;
    GouHibiki, "gou-hibiki", 0x44;
    Dorai, "dorai", 0x45;
    Guile, "guile", 0x46;
    Dhalsim, "dhalsim", 0x47;
    Oro, "oro", 0x48;
    Damnd, "damnd", 0x49;
    Haggar, "haggar", 0x4a;
    Sodom, "sodom", 0x4b;
    Belger, "belger", 0x4c;
    Zeku, "zeku", 0x4d;
    Rose, "rose", 0x4e;
    Zangief, "zangief", 0x4f;
    Vega, "vega", 0x50;
    EvilRyu, "evil-ryu", sf_video::background_bank::EVIL_RYU_THEME;
    Ken, "ken", 0x52;
    ChunLi, "chun-li", 0x53;
    Balrog, "balrog", 0x54;
    Dan, "dan", 0x55;
    Akuma, "akuma", 0x56;
    Charlie, "charlie", 0x57;
    Goutetsu, "goutetsu", 0x58;
    Guy, "guy", 0x59;
    Cody, "cody", 0x5a;
    Rolento, "rolento", 0x5b;
    Hugo, "hugo", 0x5c;
    Abigail, "abigail", 0x5d;
    Poison, "poison", 0x5e;
}

impl From<crate::roster::TournamentFighter> for ThemeId {
    fn from(fighter: crate::roster::TournamentFighter) -> Self {
        use crate::roster::TournamentFighter::*;
        match fighter {
            Ryu => Self::Ryu,
            Ken => Self::Ken,
            Retsu => Self::Retsu,
            Geki => Self::Geki,
            Joe => Self::Joe,
            Mike => Self::Mike,
            Lee => Self::Lee,
            Gen => Self::Gen,
            Birdie => Self::Birdie,
            Eagle => Self::Eagle,
            Adon => Self::Adon,
            Sagat => Self::Sagat,
        }
    }
}

impl ThemeId {
    /// Names used by fighter data, including the older Charlie and Bison slugs.
    pub fn from_fighter_slug(slug: &str) -> Option<Self> {
        match slug {
            "nash" | "charlie-nash" => Some(Self::Charlie),
            "m-bison" => Some(Self::Bison),
            "andore" => Some(Self::Hugo),
            "test" => None,
            _ => Self::from_slug(slug),
        }
    }

    /// True for the arrangements and the test score beyond the original ROM.
    pub const fn is_extended(self) -> bool {
        self.command() >= sf_video::background_bank::EXTENDED_THEME_FIRST as u8
    }
}

/// A timed request, accompanied by the stage's original fallback command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeCue {
    pub id: ThemeId,
    pub fallback: u8,
}

/// The primary owner followed by the other members in roster order.
/// Ryu's and Ken's cursor defaults never confer membership.
pub fn stage_members(stage: &str) -> &'static [ThemeId] {
    use ThemeId::*;
    match stage {
        "retsu" => &[Retsu],
        "geki" => &[Geki],
        "joe" => &[Joe],
        "mike" => &[Mike],
        "lee" => &[Lee],
        "gen" => &[Gen],
        "birdie" => &[Birdie],
        "eagle" => &[Eagle],
        "adon" => &[Adon],
        "sagat" => &[Sagat],
        "evil-ryu" => &[EvilRyu],
        "bison" => &[Bison],
        "gouken" => &[Gouken, Akuma, Goutetsu],
        "gou-hibiki" => &[GouHibiki, Dan],
        "dorai" => &[Dorai, ChunLi],
        "guile" => &[Guile],
        "dhalsim" => &[Dhalsim],
        "oro" => &[Oro],
        "damnd" => &[Damnd, Guy, Cody],
        "haggar" => &[Haggar, Poison, Hugo],
        "sodom" => &[Sodom, Rolento],
        "belger" => &[Belger, Abigail],
        "zeku" => &[Zeku],
        "rose" => &[Rose, Charlie],
        "zangief" => &[Zangief],
        "vega" => &[Vega, Balrog],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn the_registry_has_stable_unique_assignments() {
        let mut commands = BTreeSet::new();
        for id in ThemeId::ALL {
            assert_eq!(ThemeId::from_slug(id.slug()), Some(id));
            assert_eq!(ThemeId::from_command(id.command()), Some(id));
            assert!(commands.insert(id.command()));
        }
        assert_eq!(commands.len(), 41);
        assert_eq!(ThemeId::Ryu.command(), 0x41);
        assert_eq!(ThemeId::Test.command(), 0x40);
        assert_eq!(ThemeId::EvilRyu.command(), 0x51);
        assert_eq!(ThemeId::from_fighter_slug("nash"), Some(ThemeId::Charlie));
        assert!(!commands.contains(&0x7e), "reserved for the title");
    }

    #[test]
    fn cursor_defaults_are_not_stage_memberships() {
        assert_eq!(stage_members("retsu"), [ThemeId::Retsu]);
        assert_eq!(stage_members("joe"), [ThemeId::Joe]);
        assert_eq!(stage_members("gouken"), [ThemeId::Gouken, ThemeId::Akuma, ThemeId::Goutetsu]);
        assert!(stage_members("test-stage").is_empty());
    }
}
