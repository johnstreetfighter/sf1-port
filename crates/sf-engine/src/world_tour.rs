//! 1987 World Tour campaign data.

pub mod native;

pub use crate::arcade::Country;

const LORE: &[(&str, &str, &str)] = &[
    ("Ryu & Ken vs. Akuma / Gouken", "Ryu", "Akuma"),
    ("Gouken & Retsu vs. Goutetsu", "Gouken", "Goutetsu"),
    ("Cody & Guy vs. Mike Haggar", "Cody", "Haggar"),
    ("Gen & Chun-Li vs. M. Bison", "Gen", "M. Bison"),
    ("Sagat vs. Gou Hibiki & Dan", "Sagat", "Gou Hibiki"),
];

pub fn lore_episodes(bonus_challengers: bool) -> &'static [(&'static str, &'static str, &'static str)] {
    if bonus_challengers { LORE } else { &[] }
}
