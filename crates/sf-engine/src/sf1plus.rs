//! Match switches retained in the configuration and netplay rules.

use crate::config::RulesetPreset;

/// The saved SF1+ damage, freeze-input, and throw switches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sf1PlusRules {
    /// Use the original ROM damage for the 1987 shoto specials and Tiger Shot.
    pub classic_1987_special_damage: bool,
    /// Keep attack inputs entered during a Super freeze.
    pub disable_freeze_input_lockout: bool,
    /// Classic SF2 throws: the 7-frame tech window does not open.
    pub untechable_sf2_throws: bool,
    /// Skip the wakeup separation after a corner throw.
    pub allow_corner_throw_loops: bool,
}

// Canonical gameplay schema: explicit tags and exhaustive fields (NP1).
crate::canonical::canonical_struct!(Sf1PlusRules, "Sf1PlusRules.v1", {
    classic_1987_special_damage, disable_freeze_input_lockout, untechable_sf2_throws, allow_corner_throw_loops,
});

impl Sf1PlusRules {
    pub const fn for_preset(preset: RulesetPreset) -> Self {
        let faithful = matches!(preset, RulesetPreset::Faithful1987);
        Self {
            classic_1987_special_damage: faithful,
            disable_freeze_input_lockout: !faithful,
            untechable_sf2_throws: false,
            allow_corner_throw_loops: false,
        }
    }
}
