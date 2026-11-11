//! Match configuration, ruleset presets, and Tier 1 core overhaul toggles (`#94`, `O3`).
//!
//! Defines [`RulesetPreset`], [`CoreToggles`], [`MatchConfig`], and the per-session
//! [`CoreToggleState`] used by [`crate::fighter::Program`] and [`crate::fighter::Bus`].
//!
//! Default persistence path on Windows: `%APPDATA%\sf1-port\match_config.toml`.
//! When no configuration file exists yet, [`MatchConfig::default`] selects
//! [`RulesetPreset::DefinitiveOverhaul`] (as required once the faithful port is
//! complete). MAME 0.289 comparison fixtures use [`MatchConfig::faithful`]
//! ([`RulesetPreset::Faithful1987`]), where every overhaul toggle is `false` and
//! execution is bit-for-bit identical to the 1987 arcade ROM.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The keys of the two rebalance switches that Balanced roster absorbed (SP1,
/// `#247`): symmetric shotos with a blockable single-hit Shoryuken, and balanced
/// damage and chip. Old settings files carry them.
const LEGACY_REBALANCE_KEYS: [&str; 2] = [
    "symmetric_shotos_and_blockable_dp",
    "balanced_damage_and_chip",
];

/// A toggle's boolean value in the file or on the command line: `true`, `1`,
/// `on` or `yes` and their opposites.
fn parse_toggle_bool(key: &str, value: &str) -> Result<bool, String> {
    match value {
        "true" | "1" | "on" | "yes" => Ok(true),
        "false" | "0" | "off" | "no" => Ok(false),
        _ => Err(format!("expected boolean for '{key}', got '{value}'")),
    }
}

/// Default window (in 60 Hz frames) before hitstun, blockstun, or attack recovery
/// ends during which [`CoreToggles::recovery_input_buffer`] retains normal and
/// special attack inputs (`#94`, `O3`).
pub const DEFAULT_RECOVERY_BUFFER_WINDOW: u8 = 4;

/// Default total motion window (in 60 Hz frames) for *Hadoken*, *Shoryuken*, and
/// *Tatsumaki Senpukyaku* command trackers when [`CoreToggles::lenient_command_trackers`]
/// is enabled (`#95`, `O4`; original 1987 window is `0x12` = 18 frames).
pub const DEFAULT_COMMAND_TRACKER_WINDOW: u8 = 28;

/// Default directional release grace window (in 60 Hz frames) after releasing the
/// final direction of a completed special command when [`CoreToggles::lenient_command_trackers`]
/// is enabled (`#95`, `O4`; original 1987 grace is 0 frames).
pub const DEFAULT_COMMAND_RELEASE_GRACE: u8 = 8;

/// Default human-like reaction delay (in 60 Hz frames) before the CPU's `0x45630`
/// follow-up decision may take the `0x4571a` input-reading reaction branch when
/// [`CoreToggles::fair_cpu_and_sagat_scaling`] is enabled (`#97`, `O6`; original
/// 1987 delay is 0 frames / immediate frame-1 input reading).
pub const DEFAULT_CPU_REACTION_DELAY: u8 = 8;

/// Top-level ruleset presets selectable in the overhaul suite (`#94`, `O3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RulesetPreset {
    /// Enables all Tier 1 core overhaul toggles while preserving the 1987 game's
    /// fundamental ruleset. Default preset once the faithful port is complete.
    #[default]
    DefinitiveOverhaul,
    /// Disables all overhaul toggles (`false`) for 100% MAME 0.289 parity.
    Faithful1987,
    /// The classic level: the control switches on (positive-edge attacks, input
    /// buffering, lenient commands, interruptible walks, anytime jump attacks,
    /// standard high and low blocking) under 1987's rules, with Balanced roster
    /// and the three modern rules off. Called SF1++ Competitive until 2026-11-07.
    ClassicCompetitive,
    /// Enables responsive controls, blocking, and walk cycle while keeping
    /// special-move command trackers at original 1987 strictness for neutral footsies.
    PureFootsies1987Plus,
    /// User-defined combination of individual toggle overrides.
    Custom,
}

impl RulesetPreset {
    /// All five ruleset presets in canonical order.
    pub const ALL: [RulesetPreset; 5] = [
        RulesetPreset::DefinitiveOverhaul,
        RulesetPreset::Faithful1987,
        RulesetPreset::ClassicCompetitive,
        RulesetPreset::PureFootsies1987Plus,
        RulesetPreset::Custom,
    ];

    /// Machine-readable TOML / CLI identifier for this preset.
    pub const fn slug(self) -> &'static str {
        match self {
            RulesetPreset::DefinitiveOverhaul => "definitive_overhaul",
            RulesetPreset::Faithful1987 => "faithful_1987",
            RulesetPreset::ClassicCompetitive => "classic_competitive",
            RulesetPreset::PureFootsies1987Plus => "pure_footsies_1987_plus",
            RulesetPreset::Custom => "custom",
        }
    }

    /// Human-readable display name for this preset.
    pub const fn display_name(self) -> &'static str {
        match self {
            RulesetPreset::DefinitiveOverhaul => "Definitive Overhaul",
            RulesetPreset::Faithful1987 => "Faithful 1987",
            RulesetPreset::ClassicCompetitive => "Classic Competitive",
            RulesetPreset::PureFootsies1987Plus => "Pure Footsies (1987+)",
            RulesetPreset::Custom => "Custom",
        }
    }

    /// The saved key and the screen name Classic Competitive had before the
    /// rename (SP1, `#247`). A settings file naming either still loads, as
    /// [`RulesetPreset::ClassicCompetitive`].
    pub const LEGACY_CLASSIC_COMPETITIVE_NAMES: [&'static str; 2] =
        ["sf1_plus_plus_competitive", "SF1++ Competitive"];

    /// Parse a preset from either its [`Self::slug`] or case-insensitive
    /// [`Self::display_name`], or from one of the names Classic Competitive had
    /// before its rename ([`Self::LEGACY_CLASSIC_COMPETITIVE_NAMES`]).
    pub fn from_str_name(s: &str) -> Result<Self, String> {
        let trimmed = s.trim();
        if Self::LEGACY_CLASSIC_COMPETITIVE_NAMES
            .iter()
            .any(|old| old.eq_ignore_ascii_case(trimmed))
        {
            return Ok(RulesetPreset::ClassicCompetitive);
        }
        for preset in Self::ALL {
            if preset.slug().eq_ignore_ascii_case(trimmed)
                || preset.display_name().eq_ignore_ascii_case(trimmed)
            {
                return Ok(preset);
            }
        }
        Err(format!("unknown ruleset preset '{trimmed}'"))
    }

    /// Cycle to the next standard ruleset preset (for live hotkey cycling).
    pub const fn next_cycle(self) -> Self {
        match self {
            RulesetPreset::DefinitiveOverhaul => RulesetPreset::Faithful1987,
            RulesetPreset::Faithful1987 => RulesetPreset::ClassicCompetitive,
            RulesetPreset::ClassicCompetitive => RulesetPreset::PureFootsies1987Plus,
            RulesetPreset::PureFootsies1987Plus | RulesetPreset::Custom => {
                RulesetPreset::DefinitiveOverhaul
            }
        }
    }
}

/// Tier 1 core overhaul toggles (`#94` `O3` through `#97` `O6`).
///
/// Every toggle is an independent, deterministic gate at the translated routine's
/// entry point. When all boolean toggles are `false` ([`RulesetPreset::Faithful1987`]),
/// the engine executes the exact 1987 arcade paths with zero diffs against MAME 0.289.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreToggles {
    /// Trigger normal and special attacks on button press (`F+1`) rather than waiting
    /// for button release, and prevent 255-frame (`0xff`) hold saturation lockouts (`#94`, `O3`).
    pub positive_edge_attacks: bool,
    /// Optional sub-toggle for `positive_edge_attacks`: also trigger special moves on
    /// button release (`#94`, `O3`).
    pub negative_edge_specials: bool,
    /// Retain normal and special attack inputs entered within `recovery_buffer_window`
    /// frames before hitstun, blockstun, or attack recovery ends and execute them on
    /// the first actionable frame (`#94`, `O3`).
    pub recovery_input_buffer: bool,
    /// Configurable window in frames (`0..=30`, default [`DEFAULT_RECOVERY_BUFFER_WINDOW`] = 4)
    /// for `recovery_input_buffer` (`#94`, `O3`).
    pub recovery_buffer_window: u8,
    /// Lenient special command trackers: configurable motion window, directional
    /// release grace window, and diagonal *Shoryuken* shortcuts (`#95`, `O4`).
    pub lenient_command_trackers: bool,
    /// Configurable motion window in frames (`1..=60`, default [`DEFAULT_COMMAND_TRACKER_WINDOW`] = 28)
    /// for `lenient_command_trackers` (`#95`, `O4`).
    pub command_tracker_window: u8,
    /// Configurable directional release grace window in frames (`0..=30`, default
    /// [`DEFAULT_COMMAND_RELEASE_GRACE`] = 8) for `lenient_command_trackers` (`#95`, `O4`).
    pub command_release_grace: u8,
    /// Allow special moves, normal attacks, blocks, crouches, and jumps to interrupt
    /// gated walk frames (`0x91`) immediately, and suppress the 16-pixel first-frame
    /// backward shift when starting a back walk (`#95`, `O4`).
    pub interruptible_walk_cycle: bool,
    /// Allow neutral and directional jump attacks during ascent, apex, or descent
    /// (up to 4 frames before landing) (`#95`, `O4`).
    pub anytime_jump_attacks: bool,
    /// Sub-toggle for `anytime_jump_attacks`: shorten pre-jump crouch startup from
    /// 9 frames (`0..=8`) to 4 frames (`0..=3`) (`#95`, `O4`).
    pub fast_prejump_crouch: bool,
    /// Unconditional back/down-back guard, low-block vs jump attacks, and symmetric P1/P2 block (`#96`, `O5`).
    pub standard_high_low_block: bool,
    /// Balanced roster (SP1, `#247`; design 5.3): the tuning layer that evens out
    /// the roster with numbers only. Its first content is the two rebalance
    /// switches it absorbed, so the game never rebalances twice: symmetric P1/P2
    /// forward-punch priority and shoto frames with a blockable single-hit
    /// Shoryuken (`#96`, `O5`), and rebalanced special damage, multi-hit
    /// knockdown scaling and normalized stage chip damage (`#97`, `O6`). Off, the
    /// ROM's own numbers.
    pub balanced_roster: bool,
    /// Modern rule: true combos (SP17, `#261`): a fighter in a ground hit
    /// reaction can be hit, with hitstun and blockstun from the attacking
    /// move's data, one hit per move, and combo damage scaling
    /// ([`crate::combo`]). Off, the 1987 reaction: invulnerable throughout.
    pub true_combos: bool,
    /// Modern rule: cancels (SP18, `#262`): a special's command cancels a
    /// normal the move data marks cancelable, inside the window its hit or
    /// guard opens ([`crate::combo::cancel`]). Off, 1987: nothing starts
    /// until the move ends.
    pub cancels: bool,
    /// Modern rule: throws and throw techs (SP19, `#263`): close-range
    /// `Forward/Back + HP/HK` throws, the victim's being-thrown arc and
    /// get-up, a 7-frame throw tech window, and post-wakeup throw
    /// invulnerability ([`crate::combo::throw`]). Off, 1987: no throws.
    pub throws: bool,
    /// Sub-toggle for `balanced_roster`: allow chip damage to KO at 1 HP (default `false`) (`#97`, `O6`).
    pub chip_death: bool,
    /// Suppress `SCENE_MODE |= 0x20` slow-motion on non-lethal unblocked projectile hits (`#97`, `O6`).
    pub disable_fireball_slowdown: bool,
    /// 60-frame real-second timer ticks and Sudden Death / Draw decider round on tied matches (`#97`, `O6`).
    pub real_second_timer_and_draws: bool,
    /// CPU input-reading reaction delay, Retsu unblockable fix, and Sagat/Adon difficulty scaling (`#97`, `O6`).
    pub fair_cpu_and_sagat_scaling: bool,
    /// Configurable CPU input-reading reaction delay in frames (`0..=30`, default
    /// [`DEFAULT_CPU_REACTION_DELAY`] = 8) for `fair_cpu_and_sagat_scaling` (`#97`, `O6`).
    pub cpu_reaction_delay: u8,
}

// Canonical gameplay schema: explicit tags and exhaustive fields (NP1).
crate::canonical::canonical_struct!(CoreToggles, "CoreToggles.v1", {
    positive_edge_attacks, negative_edge_specials, recovery_input_buffer, recovery_buffer_window,
    lenient_command_trackers, command_tracker_window, command_release_grace, interruptible_walk_cycle,
    anytime_jump_attacks, fast_prejump_crouch, standard_high_low_block, balanced_roster, true_combos,
    cancels, throws, chip_death, disable_fireball_slowdown, real_second_timer_and_draws,
    fair_cpu_and_sagat_scaling, cpu_reaction_delay,
});

crate::canonical::canonical_struct!(TourRules, "TourRules.v1", {
    rpg, projectile_only, no_blocking, survival_waves, time_attack_frames, dramatic, cross_era,
});

crate::canonical::canonical_struct!(RecoveryBufferSlot, "RecoveryBufferSlot.v1", {
    punch, kick, d0, hadoken_done, shoryuken_done, tatsumaki_done, age, special_press,
});

crate::canonical::canonical_struct!(SpecialPress, "SpecialPress.v1", {
    d0, done,
});

crate::canonical::canonical_enum!(RulesetPreset, "RulesetPreset.v1";
    DefinitiveOverhaul => 0,
    Faithful1987 => 1,
    ClassicCompetitive => 2,
    PureFootsies1987Plus => 3,
    Custom => 4,
);


impl crate::canonical::Canonical for MatchConfig {
    fn encode(&self, out: &mut crate::canonical::CanonicalHasher) {
        let Self {
            preset, toggles, roster, fill_meters, p2_fair_cpu,
            p1_scheme: _, p2_scheme: _, modern_control_damage_penalty,
            sf1plus, gauge_mode, disable_comeback_scaling, disable_raw_drive_rush,
            tour, dramatic, difficulty, round_time,
        } = self;
        out.tag("MatchConfig.v2");
        preset.encode(out);
        toggles.encode(out);
        roster.encode(out);
        fill_meters.encode(out);
        p2_fair_cpu.encode(out);
        modern_control_damage_penalty.encode(out);
        sf1plus.encode(out);
        gauge_mode.encode(out);
        disable_comeback_scaling.encode(out);
        disable_raw_drive_rush.encode(out);
        tour.encode(out);
        dramatic.encode(out);
        difficulty.encode(out);
        round_time.encode(out);
        // Scheme selectors currently affect no translated-machine input.
        // TRD1 must normalize/synchronize their effects before making them live.
    }
}

impl crate::canonical::Canonical for CoreToggleState {
    fn encode(&self, out: &mut crate::canonical::CanonicalHasher) {
        let Self {
            neg_edge_latch, recovery_buffer, tracker_release_age, dp_connected,
            human_attack_frames, air_adj_frames_1, air_adj_rem_1,
            air_adj_frames_2, air_adj_rem_2, combo, cancel, throw, dramatic, era,
            missing_ryu_theme: _,
        } = self;
        out.tag("CoreToggleState.v3");
        neg_edge_latch.encode(out);
        recovery_buffer.encode(out);
        tracker_release_age.encode(out);
        dp_connected.encode(out);
        human_attack_frames.encode(out);
        air_adj_frames_1.encode(out);
        air_adj_rem_1.encode(out);
        air_adj_frames_2.encode(out);
        air_adj_rem_2.encode(out);
        combo.encode(out);
        cancel.encode(out);
        throw.encode(out);
        dramatic.encode(out);
        era.encode(out);
        // A pending music-pack fallback log note is local diagnostic state.
    }
}

impl CoreToggles {
    /// Canonical toggle set for a given [`RulesetPreset`].
    pub const fn for_preset(preset: RulesetPreset) -> Self {
        match preset {
            RulesetPreset::Faithful1987 => Self {
                positive_edge_attacks: false,
                negative_edge_specials: false,
                recovery_input_buffer: false,
                recovery_buffer_window: DEFAULT_RECOVERY_BUFFER_WINDOW,
                lenient_command_trackers: false,
                command_tracker_window: DEFAULT_COMMAND_TRACKER_WINDOW,
                command_release_grace: DEFAULT_COMMAND_RELEASE_GRACE,
                interruptible_walk_cycle: false,
                anytime_jump_attacks: false,
                fast_prejump_crouch: false,
                standard_high_low_block: false,
                balanced_roster: false,
                true_combos: false,
                cancels: false,
                throws: false,
                chip_death: false,
                disable_fireball_slowdown: false,
                real_second_timer_and_draws: false,
                fair_cpu_and_sagat_scaling: false,
                cpu_reaction_delay: DEFAULT_CPU_REACTION_DELAY,
            },
            RulesetPreset::DefinitiveOverhaul | RulesetPreset::Custom => Self {
                positive_edge_attacks: true,
                negative_edge_specials: true,
                recovery_input_buffer: true,
                recovery_buffer_window: DEFAULT_RECOVERY_BUFFER_WINDOW,
                lenient_command_trackers: true,
                command_tracker_window: DEFAULT_COMMAND_TRACKER_WINDOW,
                command_release_grace: DEFAULT_COMMAND_RELEASE_GRACE,
                interruptible_walk_cycle: true,
                anytime_jump_attacks: true,
                fast_prejump_crouch: true,
                standard_high_low_block: true,
                balanced_roster: true,
                true_combos: true,
                cancels: true,
                throws: true,
                chip_death: false,
                disable_fireball_slowdown: true,
                real_second_timer_and_draws: true,
                fair_cpu_and_sagat_scaling: true,
                cpu_reaction_delay: DEFAULT_CPU_REACTION_DELAY,
            },
            RulesetPreset::ClassicCompetitive => Self {
                positive_edge_attacks: true,
                negative_edge_specials: true,
                recovery_input_buffer: true,
                recovery_buffer_window: DEFAULT_RECOVERY_BUFFER_WINDOW,
                lenient_command_trackers: true,
                command_tracker_window: DEFAULT_COMMAND_TRACKER_WINDOW,
                command_release_grace: DEFAULT_COMMAND_RELEASE_GRACE,
                interruptible_walk_cycle: true,
                anytime_jump_attacks: true,
                fast_prejump_crouch: true,
                standard_high_low_block: true,
                balanced_roster: false,
                true_combos: false,
                cancels: false,
                throws: false,
                chip_death: false,
                disable_fireball_slowdown: true,
                real_second_timer_and_draws: true,
                fair_cpu_and_sagat_scaling: true,
                cpu_reaction_delay: DEFAULT_CPU_REACTION_DELAY,
            },
            RulesetPreset::PureFootsies1987Plus => Self {
                positive_edge_attacks: true,
                negative_edge_specials: true,
                recovery_input_buffer: true,
                recovery_buffer_window: DEFAULT_RECOVERY_BUFFER_WINDOW,
                lenient_command_trackers: false,
                command_tracker_window: DEFAULT_COMMAND_TRACKER_WINDOW,
                command_release_grace: DEFAULT_COMMAND_RELEASE_GRACE,
                interruptible_walk_cycle: true,
                anytime_jump_attacks: true,
                fast_prejump_crouch: true,
                standard_high_low_block: true,
                balanced_roster: false,
                true_combos: false,
                cancels: false,
                throws: false,
                chip_death: false,
                disable_fireball_slowdown: true,
                real_second_timer_and_draws: true,
                fair_cpu_and_sagat_scaling: true,
                cpu_reaction_delay: DEFAULT_CPU_REACTION_DELAY,
            },
        }
    }

    /// All overhaul toggles disabled (`Faithful 1987`).
    pub const fn faithful() -> Self {
        Self::for_preset(RulesetPreset::Faithful1987)
    }

    /// Canonical `Definitive Overhaul` toggle set.
    pub const fn definitive() -> Self {
        Self::for_preset(RulesetPreset::DefinitiveOverhaul)
    }
}

impl Default for CoreToggles {
    fn default() -> Self {
        Self::definitive()
    }
}

/// Session-level match configuration combining a [`RulesetPreset`] and [`CoreToggles`] (`#94`, `O3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchConfig {
    pub preset: RulesetPreset,
    pub toggles: CoreToggles,
    /// 12-fighter roster, ban list, and country start (`#98`, `O7`).
    pub roster: crate::roster::RosterSettings,
    /// Training's `FILL METERS` (ADR-0013, spec #238, TR7). In a Training
    /// fight with it on, every era resource behind a gauge (super stocks,
    /// level gauges, Drive, V-Gauge and the like) is held at its maximum
    /// and refilled straight after it is spent, spending working normally.
    /// It acts on the real resource, never on the drawing alone. Nothing
    /// reads it until the era resources reach real fights (project 4, spec
    /// #91). Off in every preset and outside Training: the session stamps
    /// it from `settings.toml` when a Training fight starts, so it is not
    /// part of the match-config file and no preset changes it.
    pub fill_meters: bool,
    /// Player 2's pad is played by the fair CPU (SP21, `#264`;
    /// [`crate::fair_cpu`]): Training against the CPU under an overhaul
    /// preset. The session stamps it when such a fight starts, so, like
    /// `fill_meters`, it is not part of the match-config file and no preset
    /// changes it. The machine ignores it under Faithful 1987, which keeps
    /// Capcom's CPU.
    pub p2_fair_cpu: bool,
    /// Per-player cabinet scheme (`#100`, `O9`).
    pub p1_scheme: crate::session_io::ControlScheme,
    pub p2_scheme: crate::session_io::ControlScheme,
    /// 20% damage and +2 startup on modern 1-button neutral specials and supers.
    /// On for the competitive preset, off otherwise (`#100`, `O9`).
    pub modern_control_damage_penalty: bool,
    /// SF1+ throw, damage, and freeze rules (`#101`, `O10`).
    pub sf1plus: crate::sf1plus::Sf1PlusRules,
    /// Which era gauge a match draws (`#103`, `O12`). Pure Footsies forces gauges off.
    pub gauge_mode: crate::era::GaugeMode,
    /// USF4 Ultra Double deals full damage instead of the scaled value (`#106`, `O15`).
    pub disable_comeback_scaling: bool,
    /// Refuse the 1-bar raw neutral Drive Rush. Cancel Drive Rush stays (`#108`, `O17`).
    pub disable_raw_drive_rush: bool,
    /// World Tour mission rules. Off for every preset (`#111`, `O20`).
    pub tour: TourRules,
    /// Session-only rules for a native Dramatic fight.
    pub dramatic: Option<crate::dramatic::DramaticRules>,
    /// The front end's Gameplay `DIFFICULTY` (1 Easy, 2 Normal, 3 Difficult,
    /// 4 Very Difficult) and `ROUND TIME` (100, 150, 200, 250). The 1987
    /// program never reads them: the front end powers every fight on with
    /// them as DIP words (`[dips]` in the file; factory 2 and 100). The fair
    /// CPU reads `difficulty` for its level ([`crate::fair_cpu::Level`]). No
    /// preset changes them.
    pub difficulty: u8,
    pub round_time: u16,
}

/// Mission switches written onto a match from the 1987 World Tour (`#111`, `O20`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TourRules {
    pub rpg: bool,
    pub projectile_only: bool,
    pub no_blocking: bool,
    pub survival_waves: u8,
    pub time_attack_frames: u16,
    pub dramatic: bool,
    pub cross_era: bool,
}

impl TourRules {
    pub const fn off() -> Self {
        Self {
            rpg: false,
            projectile_only: false,
            no_blocking: false,
            survival_waves: 0,
            time_attack_frames: 0,
            dramatic: false,
            cross_era: false,
        }
    }
}

impl Default for TourRules {
    fn default() -> Self {
        Self::off()
    }
}

impl Default for MatchConfig {
    /// Defaults to [`RulesetPreset::DefinitiveOverhaul`] once the faithful port is complete.
    fn default() -> Self {
        Self::from_preset(RulesetPreset::DefinitiveOverhaul)
    }
}

/// Default Windows path `%APPDATA%\sf1-port\match_config.toml` (`#94`, `O3`).
pub fn default_match_config_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("sf1-port").join("match_config.toml")
}

impl MatchConfig {
    /// Whether Character Select offers the era, revision, and loadout choices.
    pub const fn era_select_enabled(&self) -> bool {
        matches!(self.preset, RulesetPreset::DefinitiveOverhaul)
            || (matches!(self.preset, RulesetPreset::Custom) && self.toggles.balanced_roster)
    }

    /// Construct a [`MatchConfig`] from one of the five [`RulesetPreset`]s.
    pub const fn from_preset(preset: RulesetPreset) -> Self {
        Self {
            preset,
            toggles: CoreToggles::for_preset(preset),
            roster: crate::roster::RosterSettings::for_preset(preset),
            fill_meters: false,
            p2_fair_cpu: false,
            p1_scheme: crate::session_io::ControlScheme::Classic,
            p2_scheme: crate::session_io::ControlScheme::Classic,
            modern_control_damage_penalty: matches!(preset, RulesetPreset::ClassicCompetitive),
            sf1plus: crate::sf1plus::Sf1PlusRules::for_preset(preset),
            gauge_mode: crate::era::GaugeMode::for_preset(preset),
            disable_comeback_scaling: false,
            disable_raw_drive_rush: false,
            tour: TourRules::off(),
            dramatic: None,
            difficulty: 2,
            round_time: 100,
        }
    }

    /// Construct the `Faithful 1987` configuration (all overhaul toggles `false`).
    pub const fn faithful() -> Self {
        Self::from_preset(RulesetPreset::Faithful1987)
    }

    /// Construct the `Definitive Overhaul` configuration.
    pub const fn definitive() -> Self {
        Self::from_preset(RulesetPreset::DefinitiveOverhaul)
    }

    /// Switch to `preset` and load its canonical [`CoreToggles`].
    pub fn apply_preset(&mut self, preset: RulesetPreset) {
        self.preset = preset;
        if preset != RulesetPreset::Custom {
            self.toggles = CoreToggles::for_preset(preset);
            self.roster = crate::roster::RosterSettings::for_preset(preset);
            self.p1_scheme = crate::session_io::ControlScheme::Classic;
            self.p2_scheme = crate::session_io::ControlScheme::Classic;
            self.modern_control_damage_penalty =
                matches!(preset, RulesetPreset::ClassicCompetitive);
            self.sf1plus = crate::sf1plus::Sf1PlusRules::for_preset(preset);
            self.gauge_mode = crate::era::GaugeMode::for_preset(preset);
            self.disable_comeback_scaling = false;
            self.disable_raw_drive_rush = false;
            self.tour = TourRules::off();
        }
    }

    /// Cycle to the next standard [`RulesetPreset`] and return the newly active preset.
    pub fn cycle_preset(&mut self) -> RulesetPreset {
        let next = self.preset.next_cycle();
        self.apply_preset(next);
        next
    }

    /// Transition `preset` to [`RulesetPreset::Custom`] if `toggles` diverge from
    /// the current preset's canonical toggle set.
    pub fn sync_preset(&mut self) {
        if self.preset != RulesetPreset::Custom
            && self.toggles != CoreToggles::for_preset(self.preset)
        {
            self.preset = RulesetPreset::Custom;
        }
    }

    /// Builder: override `positive_edge_attacks` and update `preset` to `Custom` if needed.
    pub fn with_positive_edge_attacks(mut self, enabled: bool) -> Self {
        self.set_positive_edge_attacks(enabled);
        self
    }

    /// Builder: override `negative_edge_specials` and update `preset` to `Custom` if needed.
    pub fn with_negative_edge_specials(mut self, enabled: bool) -> Self {
        self.set_negative_edge_specials(enabled);
        self
    }

    /// Builder: override `recovery_input_buffer` and update `preset` to `Custom` if needed.
    pub fn with_recovery_input_buffer(mut self, enabled: bool) -> Self {
        self.set_recovery_input_buffer(enabled);
        self
    }

    /// Builder: override `recovery_buffer_window` and update `preset` to `Custom` if needed.
    pub fn with_recovery_buffer_window(mut self, window: u8) -> Self {
        self.set_recovery_buffer_window(window);
        self
    }

    /// Set `positive_edge_attacks` and transition to [`RulesetPreset::Custom`] if needed.
    pub fn set_positive_edge_attacks(&mut self, enabled: bool) {
        self.toggles.positive_edge_attacks = enabled;
        self.sync_preset();
    }

    /// Set `negative_edge_specials` and transition to [`RulesetPreset::Custom`] if needed.
    pub fn set_negative_edge_specials(&mut self, enabled: bool) {
        self.toggles.negative_edge_specials = enabled;
        self.sync_preset();
    }

    /// Set `recovery_input_buffer` and transition to [`RulesetPreset::Custom`] if needed.
    pub fn set_recovery_input_buffer(&mut self, enabled: bool) {
        self.toggles.recovery_input_buffer = enabled;
        self.sync_preset();
    }

    /// Set `recovery_buffer_window` (`0..=30`) and transition to [`RulesetPreset::Custom`] if needed.
    pub fn set_recovery_buffer_window(&mut self, window: u8) {
        self.toggles.recovery_buffer_window = window.min(30);
        self.sync_preset();
    }

    /// Builder: override `lenient_command_trackers` and update `preset` to `Custom` if needed (`#95`, `O4`).
    pub fn with_lenient_command_trackers(mut self, enabled: bool) -> Self {
        self.set_lenient_command_trackers(enabled);
        self
    }

    /// Builder: override `command_tracker_window` and update `preset` to `Custom` if needed (`#95`, `O4`).
    pub fn with_command_tracker_window(mut self, window: u8) -> Self {
        self.set_command_tracker_window(window);
        self
    }

    /// Builder: override `command_release_grace` and update `preset` to `Custom` if needed (`#95`, `O4`).
    pub fn with_command_release_grace(mut self, grace: u8) -> Self {
        self.set_command_release_grace(grace);
        self
    }

    /// Builder: override `interruptible_walk_cycle` and update `preset` to `Custom` if needed (`#95`, `O4`).
    pub fn with_interruptible_walk_cycle(mut self, enabled: bool) -> Self {
        self.set_interruptible_walk_cycle(enabled);
        self
    }

    /// Builder: override `anytime_jump_attacks` and update `preset` to `Custom` if needed (`#95`, `O4`).
    pub fn with_anytime_jump_attacks(mut self, enabled: bool) -> Self {
        self.set_anytime_jump_attacks(enabled);
        self
    }

    /// Builder: override `fast_prejump_crouch` and update `preset` to `Custom` if needed (`#95`, `O4`).
    pub fn with_fast_prejump_crouch(mut self, enabled: bool) -> Self {
        self.set_fast_prejump_crouch(enabled);
        self
    }

    /// Builder: override `standard_high_low_block` and update `preset` to `Custom` if needed (`#96`, `O5`).
    pub fn with_standard_high_low_block(mut self, enabled: bool) -> Self {
        self.set_standard_high_low_block(enabled);
        self
    }

    /// Builder: override `balanced_roster` and update `preset` to `Custom` if needed (SP1, `#247`).
    pub fn with_balanced_roster(mut self, enabled: bool) -> Self {
        self.set_balanced_roster(enabled);
        self
    }

    /// Set `lenient_command_trackers` and transition to [`RulesetPreset::Custom`] if needed (`#95`, `O4`).
    pub fn set_lenient_command_trackers(&mut self, enabled: bool) {
        self.toggles.lenient_command_trackers = enabled;
        self.sync_preset();
    }

    /// Set `command_tracker_window` (`1..=60`) and transition to [`RulesetPreset::Custom`] if needed (`#95`, `O4`).
    pub fn set_command_tracker_window(&mut self, window: u8) {
        self.toggles.command_tracker_window = window.clamp(1, 60);
        self.sync_preset();
    }

    /// Set `command_release_grace` (`0..=30`) and transition to [`RulesetPreset::Custom`] if needed (`#95`, `O4`).
    pub fn set_command_release_grace(&mut self, grace: u8) {
        self.toggles.command_release_grace = grace.min(30);
        self.sync_preset();
    }

    /// Set `interruptible_walk_cycle` and transition to [`RulesetPreset::Custom`] if needed (`#95`, `O4`).
    pub fn set_interruptible_walk_cycle(&mut self, enabled: bool) {
        self.toggles.interruptible_walk_cycle = enabled;
        self.sync_preset();
    }

    /// Set `anytime_jump_attacks` and transition to [`RulesetPreset::Custom`] if needed (`#95`, `O4`).
    pub fn set_anytime_jump_attacks(&mut self, enabled: bool) {
        self.toggles.anytime_jump_attacks = enabled;
        self.sync_preset();
    }

    /// Set `fast_prejump_crouch` and transition to [`RulesetPreset::Custom`] if needed (`#95`, `O4`).
    pub fn set_fast_prejump_crouch(&mut self, enabled: bool) {
        self.toggles.fast_prejump_crouch = enabled;
        self.sync_preset();
    }

    /// Set `standard_high_low_block` and transition to [`RulesetPreset::Custom`] if needed (`#96`, `O5`).
    pub fn set_standard_high_low_block(&mut self, enabled: bool) {
        self.toggles.standard_high_low_block = enabled;
        self.sync_preset();
    }

    /// Set `balanced_roster` and transition to [`RulesetPreset::Custom`] if needed (SP1, `#247`).
    pub fn set_balanced_roster(&mut self, enabled: bool) {
        self.toggles.balanced_roster = enabled;
        self.sync_preset();
    }

    /// Set `true_combos` and transition to [`RulesetPreset::Custom`] if needed (SP1, `#247`).
    pub fn set_true_combos(&mut self, enabled: bool) {
        self.toggles.true_combos = enabled;
        self.sync_preset();
    }

    /// Set `cancels` and transition to [`RulesetPreset::Custom`] if needed (SP1, `#247`).
    pub fn set_cancels(&mut self, enabled: bool) {
        self.toggles.cancels = enabled;
        self.sync_preset();
    }

    /// Set `throws` and transition to [`RulesetPreset::Custom`] if needed (SP1, `#247`).
    pub fn set_throws(&mut self, enabled: bool) {
        self.toggles.throws = enabled;
        self.sync_preset();
    }

    /// Builder: override `true_combos` and update `preset` to `Custom` if needed (SP1, `#247`).
    pub fn with_true_combos(mut self, enabled: bool) -> Self {
        self.set_true_combos(enabled);
        self
    }

    /// Builder: override `cancels` and update `preset` to `Custom` if needed (SP1, `#247`).
    pub fn with_cancels(mut self, enabled: bool) -> Self {
        self.set_cancels(enabled);
        self
    }

    /// Builder: override `throws` and update `preset` to `Custom` if needed (SP1, `#247`).
    pub fn with_throws(mut self, enabled: bool) -> Self {
        self.set_throws(enabled);
        self
    }

    /// Builder: override `chip_death` and update `preset` to `Custom` if needed (`#97`, `O6`).
    pub fn with_chip_death(mut self, enabled: bool) -> Self {
        self.set_chip_death(enabled);
        self
    }

    /// Builder: override `disable_fireball_slowdown` and update `preset` to `Custom` if needed (`#97`, `O6`).
    pub fn with_disable_fireball_slowdown(mut self, enabled: bool) -> Self {
        self.set_disable_fireball_slowdown(enabled);
        self
    }

    /// Builder: override `real_second_timer_and_draws` and update `preset` to `Custom` if needed (`#97`, `O6`).
    pub fn with_real_second_timer_and_draws(mut self, enabled: bool) -> Self {
        self.set_real_second_timer_and_draws(enabled);
        self
    }

    /// Builder: override `fair_cpu_and_sagat_scaling` and update `preset` to `Custom` if needed (`#97`, `O6`).
    pub fn with_fair_cpu_and_sagat_scaling(mut self, enabled: bool) -> Self {
        self.set_fair_cpu_and_sagat_scaling(enabled);
        self
    }

    /// Builder: override `cpu_reaction_delay` (`0..=30`) and update `preset` to `Custom` if needed (`#97`, `O6`).
    pub fn with_cpu_reaction_delay(mut self, delay: u8) -> Self {
        self.set_cpu_reaction_delay(delay);
        self
    }

    /// Set `chip_death` and transition to [`RulesetPreset::Custom`] if needed (`#97`, `O6`).
    pub fn set_chip_death(&mut self, enabled: bool) {
        self.toggles.chip_death = enabled;
        self.sync_preset();
    }

    /// Set `disable_fireball_slowdown` and transition to [`RulesetPreset::Custom`] if needed (`#97`, `O6`).
    pub fn set_disable_fireball_slowdown(&mut self, enabled: bool) {
        self.toggles.disable_fireball_slowdown = enabled;
        self.sync_preset();
    }

    /// Set `real_second_timer_and_draws` and transition to [`RulesetPreset::Custom`] if needed (`#97`, `O6`).
    pub fn set_real_second_timer_and_draws(&mut self, enabled: bool) {
        self.toggles.real_second_timer_and_draws = enabled;
        self.sync_preset();
    }

    /// Set `fair_cpu_and_sagat_scaling` and transition to [`RulesetPreset::Custom`] if needed (`#97`, `O6`).
    pub fn set_fair_cpu_and_sagat_scaling(&mut self, enabled: bool) {
        self.toggles.fair_cpu_and_sagat_scaling = enabled;
        self.sync_preset();
    }

    /// Set `cpu_reaction_delay` (`0..=30`) and transition to [`RulesetPreset::Custom`] if needed (`#97`, `O6`).
    pub fn set_cpu_reaction_delay(&mut self, delay: u8) {
        self.toggles.cpu_reaction_delay = delay.min(30);
        self.sync_preset();
    }

    /// Set a named toggle or parameter (`key = value`) and transition to [`RulesetPreset::Custom`]
    /// if the resulting toggles diverge from the active preset.
    pub fn set_field_by_name(&mut self, key: &str, value: &str) -> Result<(), String> {
        let k = key.trim();
        let v = value.trim();
        let parse_bool = |s: &str| parse_toggle_bool(k, s);
        match k {
            "positive_edge_attacks" => self.toggles.positive_edge_attacks = parse_bool(v)?,
            "negative_edge_specials" => self.toggles.negative_edge_specials = parse_bool(v)?,
            "recovery_input_buffer" => self.toggles.recovery_input_buffer = parse_bool(v)?,
            "recovery_buffer_window" => {
                let w: u8 = v
                    .parse()
                    .map_err(|_| format!("expected u8 integer for '{k}', got '{v}'"))?;
                if w > 30 {
                    return Err(format!(
                        "recovery_buffer_window must be in 0..=30 frames, got {w}"
                    ));
                }
                self.toggles.recovery_buffer_window = w;
            }
            "lenient_command_trackers" => self.toggles.lenient_command_trackers = parse_bool(v)?,
            "command_tracker_window" => {
                let w: u8 = v
                    .parse()
                    .map_err(|_| format!("expected u8 integer for '{k}', got '{v}'"))?;
                if !(1..=60).contains(&w) {
                    return Err(format!(
                        "command_tracker_window must be in 1..=60 frames, got {w}"
                    ));
                }
                self.toggles.command_tracker_window = w;
            }
            "command_release_grace" => {
                let g: u8 = v
                    .parse()
                    .map_err(|_| format!("expected u8 integer for '{k}', got '{v}'"))?;
                if g > 30 {
                    return Err(format!(
                        "command_release_grace must be in 0..=30 frames, got {g}"
                    ));
                }
                self.toggles.command_release_grace = g;
            }
            "interruptible_walk_cycle" => self.toggles.interruptible_walk_cycle = parse_bool(v)?,
            "anytime_jump_attacks" => self.toggles.anytime_jump_attacks = parse_bool(v)?,
            "fast_prejump_crouch" => self.toggles.fast_prejump_crouch = parse_bool(v)?,
            "standard_high_low_block" => self.toggles.standard_high_low_block = parse_bool(v)?,
            "balanced_roster" => self.toggles.balanced_roster = parse_bool(v)?,
            "true_combos" => self.toggles.true_combos = parse_bool(v)?,
            "cancels" => self.toggles.cancels = parse_bool(v)?,
            "throws" => self.toggles.throws = parse_bool(v)?,
            "chip_death" => self.toggles.chip_death = parse_bool(v)?,
            "disable_fireball_slowdown" => self.toggles.disable_fireball_slowdown = parse_bool(v)?,
            "real_second_timer_and_draws" => {
                self.toggles.real_second_timer_and_draws = parse_bool(v)?
            }
            "fair_cpu_and_sagat_scaling" => {
                self.toggles.fair_cpu_and_sagat_scaling = parse_bool(v)?
            }
            "cpu_reaction_delay" => {
                let d: u8 = v
                    .parse()
                    .map_err(|_| format!("expected u8 integer for '{k}', got '{v}'"))?;
                if d > 30 {
                    return Err(format!(
                        "cpu_reaction_delay must be in 0..=30 frames, got {d}"
                    ));
                }
                self.toggles.cpu_reaction_delay = d;
            }
            _ => return Err(format!("unknown toggle field '{k}'")),
        }
        self.sync_preset();
        Ok(())
    }

    /// Set one `[roster]` field. Roster choices do not retarget the ruleset preset.
    fn set_roster_field(&mut self, key: &str, value: &str) -> Result<(), String> {
        fn quoted<'a>(key: &str, value: &'a str) -> Result<&'a str, String> {
            if (value.starts_with('"') && value.ends_with('"') && value.len() >= 2)
                || (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2)
            {
                Ok(&value[1..value.len() - 1])
            } else {
                Err(format!("expected quoted string for '{key}', got '{value}'"))
            }
        }
        let flag = match value {
            "true" | "1" | "on" | "yes" => Some(true),
            "false" | "0" | "off" | "no" => Some(false),
            _ => None,
        };
        match key {
            "full_cast" => {
                self.roster.full_cast =
                    flag.ok_or_else(|| format!("expected boolean for '{key}', got '{value}'"))?
            }
            "four_country" => {
                self.roster.four_country =
                    flag.ok_or_else(|| format!("expected boolean for '{key}', got '{value}'"))?
            }
            "what_if_styles" => {
                self.roster.what_if_styles =
                    flag.ok_or_else(|| format!("expected boolean for '{key}', got '{value}'"))?
            }
            "bonus_challengers" => {
                self.roster.bonus_challengers =
                    flag.ok_or_else(|| format!("expected boolean for '{key}', got '{value}'"))?
            }
            "secret_intrusions" => {
                self.roster.secret_intrusions =
                    flag.ok_or_else(|| format!("expected boolean for '{key}', got '{value}'"))?
            }
            "ban" => self.roster.ban = crate::roster::BanFormat::from_name(quoted(key, value)?)?,
            "mode" => self.roster.mode = crate::roster::PlayMode::from_name(quoted(key, value)?)?,
            "p1" => {
                self.roster.p1 = crate::roster::TournamentFighter::from_name(quoted(key, value)?)?
            }
            "p2" => {
                self.roster.p2 = crate::roster::TournamentFighter::from_name(quoted(key, value)?)?
            }
            "country" => {
                self.roster.country =
                    crate::roster::StartingCountry::from_name(quoted(key, value)?)?
            }
            _ => return Err(format!("unknown roster field '{key}'")),
        }
        Ok(())
    }

    /// Set one `[controls]` field. Scheme choices do not retarget the ruleset preset.
    fn set_controls_field(&mut self, key: &str, value: &str) -> Result<(), String> {
        fn quoted<'a>(key: &str, value: &'a str) -> Result<&'a str, String> {
            if (value.starts_with('"') && value.ends_with('"') && value.len() >= 2)
                || (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2)
            {
                Ok(&value[1..value.len() - 1])
            } else {
                Err(format!("expected quoted string for '{key}', got '{value}'"))
            }
        }
        match key {
            "p1" => {
                self.p1_scheme = crate::session_io::ControlScheme::from_name(quoted(key, value)?)?
            }
            "p2" => {
                self.p2_scheme = crate::session_io::ControlScheme::from_name(quoted(key, value)?)?
            }
            "modern_control_damage_penalty" => {
                self.modern_control_damage_penalty = match value {
                    "true" | "1" | "on" | "yes" => true,
                    "false" | "0" | "off" | "no" => false,
                    _ => return Err(format!("expected boolean for '{key}', got '{value}'")),
                }
            }
            _ => return Err(format!("unknown controls field '{key}'")),
        }
        Ok(())
    }

    /// Set one `[sf1plus]` field. These rules do not retarget the ruleset preset.
    fn set_sf1plus_field(&mut self, key: &str, value: &str) -> Result<(), String> {
        let flag = match value {
            "true" | "1" | "on" | "yes" => true,
            "false" | "0" | "off" | "no" => false,
            _ => return Err(format!("expected boolean for '{key}', got '{value}'")),
        };
        match key {
            "classic_1987_special_damage" => self.sf1plus.classic_1987_special_damage = flag,
            "disable_freeze_input_lockout" => self.sf1plus.disable_freeze_input_lockout = flag,
            "untechable_sf2_throws" => self.sf1plus.untechable_sf2_throws = flag,
            "allow_corner_throw_loops" => self.sf1plus.allow_corner_throw_loops = flag,
            _ => return Err(format!("unknown sf1plus field '{key}'")),
        }
        Ok(())
    }

    /// Set one `[era]` field. Gauge mode does not retarget the ruleset preset.
    fn set_era_field(&mut self, key: &str, value: &str) -> Result<(), String> {
        let unquoted = value.trim().trim_matches('"').trim_matches('\'');
        match key {
            "gauge" => self.gauge_mode = crate::era::GaugeMode::from_slug(unquoted)?,
            "disable_comeback_scaling" => {
                self.disable_comeback_scaling = match unquoted {
                    "true" | "1" | "on" | "yes" => true,
                    "false" | "0" | "off" | "no" => false,
                    _ => return Err(format!("expected boolean for '{key}', got '{value}'")),
                }
            }
            "disable_raw_drive_rush" => {
                self.disable_raw_drive_rush = match unquoted {
                    "true" | "1" | "on" | "yes" => true,
                    "false" | "0" | "off" | "no" => false,
                    _ => return Err(format!("expected boolean for '{key}', got '{value}'")),
                }
            }
            _ => return Err(format!("unknown era field '{key}'")),
        }
        Ok(())
    }

    fn set_tour_field(&mut self, key: &str, value: &str) -> Result<(), String> {
        let unquoted = value.trim().trim_matches('"').trim_matches('\'');
        let flag = match unquoted {
            "true" | "1" | "on" | "yes" => Some(true),
            "false" | "0" | "off" | "no" => Some(false),
            _ => None,
        };
        match key {
            "rpg" => self.tour.rpg = flag.ok_or_else(|| format!("expected boolean for '{key}'"))?,
            "projectile_only" => {
                self.tour.projectile_only = flag.ok_or_else(|| format!("expected boolean for '{key}'"))?
            }
            "no_blocking" => {
                self.tour.no_blocking = flag.ok_or_else(|| format!("expected boolean for '{key}'"))?
            }
            "dramatic" => self.tour.dramatic = flag.ok_or_else(|| format!("expected boolean for '{key}'"))?,
            "cross_era" => self.tour.cross_era = flag.ok_or_else(|| format!("expected boolean for '{key}'"))?,
            "survival_waves" => {
                self.tour.survival_waves = unquoted.parse().map_err(|_| format!("expected integer for '{key}'"))?
            }
            "time_attack_frames" => {
                self.tour.time_attack_frames = unquoted.parse().map_err(|_| format!("expected integer for '{key}'"))?
            }
            _ => return Err(format!("unknown tour field '{key}'")),
        }
        Ok(())
    }

    /// Serialize this [`MatchConfig`] to TOML text.
    pub fn to_toml(&self) -> String {
        let t = &self.toggles;
        let mut text = format!(
            "# sf1-port match configuration (tickets #94 O3..#97 O6)\n\
             preset = \"{}\"\n\n\
             [toggles]\n\
             positive_edge_attacks = {}\n\
             negative_edge_specials = {}\n\
             recovery_input_buffer = {}\n\
             recovery_buffer_window = {}\n\
             lenient_command_trackers = {}\n\
             command_tracker_window = {}\n\
             command_release_grace = {}\n\
             interruptible_walk_cycle = {}\n\
             anytime_jump_attacks = {}\n\
             fast_prejump_crouch = {}\n\
             standard_high_low_block = {}\n\
             balanced_roster = {}\n\
             true_combos = {}\n\
             cancels = {}\n\
             throws = {}\n\
             chip_death = {}\n\
             disable_fireball_slowdown = {}\n\
             real_second_timer_and_draws = {}\n\
             fair_cpu_and_sagat_scaling = {}\n\
             cpu_reaction_delay = {}\n\
             \n\
             [roster]\n\
             full_cast = {}\n\
             four_country = {}\n\
             what_if_styles = {}\n\
             bonus_challengers = {}\n\
             secret_intrusions = {}\n\
             ban = \"{}\"\n\
             mode = \"{}\"\n\
             p1 = \"{}\"\n\
             p2 = \"{}\"\n\
             country = \"{}\"\n\
             \n\
             [controls]\n\
             p1 = \"{}\"\n\
             p2 = \"{}\"\n\
             modern_control_damage_penalty = {}\n\
             \n\
             [sf1plus]\n\
             classic_1987_special_damage = {}\n\
             disable_freeze_input_lockout = {}\n\
             untechable_sf2_throws = {}\n\
             allow_corner_throw_loops = {}\n\
             \n\
             [era]\n\
             gauge = \"{}\"\n\
             disable_comeback_scaling = {}\n\
             disable_raw_drive_rush = {}\n\
             \n\
             [tour]\n\
             rpg = {}\n\
             projectile_only = {}\n\
             no_blocking = {}\n\
             survival_waves = {}\n\
             time_attack_frames = {}\n\
             dramatic = {}\n\
             cross_era = {}\n",
            self.preset.slug(),
            t.positive_edge_attacks,
            t.negative_edge_specials,
            t.recovery_input_buffer,
            t.recovery_buffer_window,
            t.lenient_command_trackers,
            t.command_tracker_window,
            t.command_release_grace,
            t.interruptible_walk_cycle,
            t.anytime_jump_attacks,
            t.fast_prejump_crouch,
            t.standard_high_low_block,
            t.balanced_roster,
            t.true_combos,
            t.cancels,
            t.throws,
            t.chip_death,
            t.disable_fireball_slowdown,
            t.real_second_timer_and_draws,
            t.fair_cpu_and_sagat_scaling,
            t.cpu_reaction_delay,
            self.roster.full_cast,
            self.roster.four_country,
            self.roster.what_if_styles,
            self.roster.bonus_challengers,
            self.roster.secret_intrusions,
            self.roster.ban.slug(),
            self.roster.mode.slug(),
            self.roster.p1.name(),
            self.roster.p2.name(),
            self.roster.country.name(),
            self.p1_scheme.slug(),
            self.p2_scheme.slug(),
            self.modern_control_damage_penalty,
            self.sf1plus.classic_1987_special_damage,
            self.sf1plus.disable_freeze_input_lockout,
            self.sf1plus.untechable_sf2_throws,
            self.sf1plus.allow_corner_throw_loops,
            self.gauge_mode.slug(),
            self.disable_comeback_scaling,
            self.disable_raw_drive_rush,
            self.tour.rpg,
            self.tour.projectile_only,
            self.tour.no_blocking,
            self.tour.survival_waves,
            self.tour.time_attack_frames,
            self.tour.dramatic,
            self.tour.cross_era,
        );
        text.push_str(&format!(
            "\n[dips]\ndifficulty = {}\nround_time = {}\n",
            self.difficulty, self.round_time
        ));
        text
    }

    /// Parse a [`MatchConfig`] from TOML text.
    ///
    /// Loads the base [`RulesetPreset`] first (defaulting to [`RulesetPreset::DefinitiveOverhaul`]
    /// if omitted), then applies any `[toggles]` overrides. If individual overrides
    /// diverge from a non-`Custom` preset's canonical toggle set, `preset` transitions
    /// to [`RulesetPreset::Custom`].
    pub fn from_toml(text: &str) -> Result<Self, String> {
        let mut section: Option<String> = None;
        let mut preset_opt: Option<RulesetPreset> = None;
        // The file names Classic Competitive by one of its old names.
        let mut old_classic_name = false;
        let mut toggle_entries: Vec<(usize, String, String)> = Vec::new();
        let mut roster_entries: Vec<(usize, String, String)> = Vec::new();
        let mut control_entries: Vec<(usize, String, String)> = Vec::new();
        let mut sf1_entries: Vec<(usize, String, String)> = Vec::new();
        let mut era_entries: Vec<(usize, String, String)> = Vec::new();
        let mut tour_entries: Vec<(usize, String, String)> = Vec::new();
        let mut dip_entries: Vec<(usize, String, String)> = Vec::new();
        let mut seen_keys = BTreeMap::new();

        for (idx, raw_line) in text.lines().enumerate() {
            let line_num = idx + 1;
            let without_comment = match raw_line.find('#') {
                Some(pos) => &raw_line[..pos],
                None => raw_line,
            };
            let line = without_comment.trim();
            if line.is_empty() {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                let sec = line[1..line.len() - 1].trim().to_string();
                if sec != "toggles"
                    && sec != "ruleset"
                    && sec != "roster"
                    && sec != "controls"
                    && sec != "sf1plus"
                    && sec != "era"
                    && sec != "tour"
                    && sec != "dips"
                {
                    return Err(format!("line {line_num}: unknown section '[{sec}]'"));
                }
                section = Some(sec);
                continue;
            }
            let Some((raw_lhs, raw_rhs)) = line.split_once('=') else {
                return Err(format!(
                    "line {line_num}: expected 'key = value', got '{line}'"
                ));
            };
            let lhs = raw_lhs.trim();
            let rhs = raw_rhs.trim();
            if let Some(prev_line) = seen_keys.insert((section.clone(), lhs.to_string()), line_num)
            {
                return Err(format!(
                    "line {line_num}: duplicate key '{lhs}' (first defined on line {prev_line})"
                ));
            }
            if section.as_deref() == Some("roster") {
                roster_entries.push((line_num, lhs.to_string(), rhs.to_string()));
            } else if section.as_deref() == Some("controls") {
                control_entries.push((line_num, lhs.to_string(), rhs.to_string()));
            } else if section.as_deref() == Some("sf1plus") {
                sf1_entries.push((line_num, lhs.to_string(), rhs.to_string()));
            } else if section.as_deref() == Some("era") {
                era_entries.push((line_num, lhs.to_string(), rhs.to_string()));
            } else if section.as_deref() == Some("tour") {
                tour_entries.push((line_num, lhs.to_string(), rhs.to_string()));
            } else if section.as_deref() == Some("dips") {
                dip_entries.push((line_num, lhs.to_string(), rhs.to_string()));
            } else if lhs == "preset" && section.as_deref() != Some("toggles") {
                let unquoted = if (rhs.starts_with('"') && rhs.ends_with('"') && rhs.len() >= 2)
                    || (rhs.starts_with('\'') && rhs.ends_with('\'') && rhs.len() >= 2)
                {
                    &rhs[1..rhs.len() - 1]
                } else {
                    return Err(format!(
                        "line {line_num}: expected quoted preset string, got '{rhs}'"
                    ));
                };
                old_classic_name = RulesetPreset::LEGACY_CLASSIC_COMPETITIVE_NAMES
                    .iter()
                    .any(|old| old.eq_ignore_ascii_case(unquoted.trim()));
                preset_opt = Some(
                    RulesetPreset::from_str_name(unquoted)
                        .map_err(|e| format!("line {line_num}: {e}"))?,
                );
            } else {
                toggle_entries.push((line_num, lhs.to_string(), rhs.to_string()));
            }
        }

        let base_preset = preset_opt.unwrap_or(RulesetPreset::DefinitiveOverhaul);
        let mut config = Self::from_preset(base_preset);
        for (line_num, k, v) in roster_entries {
            config
                .set_roster_field(&k, &v)
                .map_err(|e| format!("line {line_num}: {e}"))?;
        }
        // A file that names the old preset got its `ban` line from the last
        // release, which wrote one into every file: not a real choice, so the
        // ban list is the new default, none. A file naming
        // `classic_competitive` keeps the list it names.
        if old_classic_name {
            config.roster.ban = crate::roster::BanFormat::Unrestricted;
        }
        for (line_num, k, v) in control_entries {
            config
                .set_controls_field(&k, &v)
                .map_err(|e| format!("line {line_num}: {e}"))?;
        }
        for (line_num, k, v) in sf1_entries {
            config
                .set_sf1plus_field(&k, &v)
                .map_err(|e| format!("line {line_num}: {e}"))?;
        }
        for (line_num, k, v) in era_entries {
            config
                .set_era_field(&k, &v)
                .map_err(|e| format!("line {line_num}: {e}"))?;
        }
        for (line_num, k, v) in tour_entries {
            config
                .set_tour_field(&k, &v)
                .map_err(|e| format!("line {line_num}: {e}"))?;
        }
        // The two rebalance switches Balanced roster absorbed (SP1, `#247`) are
        // still read from an old file. A named preset's bundle decides, so they
        // are ignored under one (an old Classic Competitive file lists both on
        // and must still load as Classic Competitive). Under Custom they carry
        // over: either one on means Balanced roster on, unless the file names
        // `balanced_roster` itself.
        let names_balanced_roster = toggle_entries
            .iter()
            .any(|(_, k, _)| k == "balanced_roster");
        let names_modern_rule = |rule: &str| toggle_entries.iter().any(|(_, k, _)| k == rule);
        let (names_combos, names_cancels, names_throws) = (
            names_modern_rule("true_combos"),
            names_modern_rule("cancels"),
            names_modern_rule("throws"),
        );
        let mut legacy_rebalance: Option<bool> = None;
        for (line_num, k, v) in toggle_entries {
            if LEGACY_REBALANCE_KEYS.contains(&k.as_str()) {
                let on = parse_toggle_bool(&k, &v).map_err(|e| format!("line {line_num}: {e}"))?;
                legacy_rebalance = Some(legacy_rebalance.unwrap_or(false) || on);
                continue;
            }
            config
                .set_field_by_name(&k, &v)
                .map_err(|e| format!("line {line_num}: {e}"))?;
        }
        if base_preset == RulesetPreset::Custom && !names_balanced_roster {
            if let Some(on) = legacy_rebalance {
                config.toggles.balanced_roster = on;
            }
        }
        // A Custom file starts from the Definitive bundle, which has the
        // modern rules on. One that names none of them (it was saved before
        // they existed) must not gain them when they get behaviour, so each
        // rule it does not name is off.
        if base_preset == RulesetPreset::Custom {
            if !names_combos {
                config.toggles.true_combos = false;
            }
            if !names_cancels {
                config.toggles.cancels = false;
            }
            if !names_throws {
                config.toggles.throws = false;
            }
        }
        for (line_num, k, v) in dip_entries {
            match (k.as_str(), v.parse::<u16>()) {
                ("difficulty", Ok(d)) if (1..=4).contains(&d) => config.difficulty = d as u8,
                ("round_time", Ok(t)) if [100, 150, 200, 250].contains(&t) => config.round_time = t,
                _ => return Err(format!("line {line_num}: bad dips entry '{k} = {v}'")),
            }
        }
        if base_preset == RulesetPreset::Custom {
            config.preset = RulesetPreset::Custom;
        }
        Ok(config)
    }

    /// Load a [`MatchConfig`] from `path`.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Self::from_toml(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Load `path`, or fall back to [`MatchConfig::default`] ([`RulesetPreset::DefinitiveOverhaul`])
    /// with a diagnostic message when `path` is missing or invalid.
    pub fn load_or_default(path: &Path) -> (Self, Option<String>) {
        match Self::load(path) {
            Ok(cfg) => (cfg, None),
            Err(reason) => (
                Self::default(),
                Some(format!(
                    "{reason}; falling back to default ruleset preset ({})",
                    RulesetPreset::DefinitiveOverhaul.display_name()
                )),
            ),
        }
    }

    /// Save this [`MatchConfig`] to `path`, creating parent directories if needed.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
            }
        }
        std::fs::write(path, self.to_toml())
            .map_err(|e| format!("cannot write {}: {e}", path.display()))
    }

    /// Compact single-line summary of the active ruleset preset and core input toggles.
    pub fn summary(&self) -> String {
        let t = &self.toggles;
        format!(
            "preset='{}' ({}) positive_edge={} neg_edge_specials={} recovery_buffer={}({}f) lenient_cmd={}({}f/{}f) walk_cancel={} air_atk={}(fast_jump={}) std_block={} balanced_roster={}(chip_ko={}) true_combos={} cancels={} throws={} no_fb_slow={} real_timer={} fair_cpu={}({}f)",
            self.preset.display_name(),
            self.preset.slug(),
            t.positive_edge_attacks,
            t.negative_edge_specials,
            t.recovery_input_buffer,
            t.recovery_buffer_window,
            t.lenient_command_trackers,
            t.command_tracker_window,
            t.command_release_grace,
            t.interruptible_walk_cycle,
            t.anytime_jump_attacks,
            t.fast_prejump_crouch,
            t.standard_high_low_block,
            t.balanced_roster,
            t.chip_death,
            t.true_combos,
            t.cancels,
            t.throws,
            t.disable_fireball_slowdown,
            t.real_second_timer_and_draws,
            t.fair_cpu_and_sagat_scaling,
            t.cpu_reaction_delay,
        )
    }
}

/// One player's buffered attack input captured during hitstun, blockstun, knockdown,
/// or attack recovery (`#94`, `O3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryBufferSlot {
    /// Latched punch strength (`0x40`, `0xe0`, `0xfe`, or `0`).
    pub punch: u8,
    /// Latched kick strength (`0x40`, `0xe0`, `0xfe`, or `0`).
    pub kick: u8,
    /// Active-high direction nibble `d0` (`0..=15`) held when the input was buffered.
    pub d0: u8,
    /// Whether the Hadoken tracker (`+0x1c`) had `done != 0` when buffered.
    pub hadoken_done: bool,
    /// Whether the Shoryuken tracker (`+0x19`) had `done != 0` when buffered.
    pub shoryuken_done: bool,
    /// Whether the Tatsumaki tracker (`+0x16`) had `done != 0` when buffered.
    pub tatsumaki_done: bool,
    /// Number of non-actionable frames elapsed since this input was buffered (`1..=window`).
    pub age: u8,
    /// Under [`CoreToggles::cancels`] (SP18, `#262`): the press as it was
    /// when it was buffered, if it fired a special there and then (`None`
    /// otherwise, and always with the switch off). The fields above go on
    /// collecting command completions after the press, for the buffer's own
    /// replay; a cancel takes only a special the press itself entered.
    pub special_press: Option<SpecialPress>,
}

/// A buffered press that fired a special when it was made
/// ([`RecoveryBufferSlot::special_press`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecialPress {
    /// The direction held (`d0`).
    pub d0: u8,
    /// The trackers' done flags then: the Tatsumaki's (`+0x16`), the
    /// Shoryuken's (`+0x19`) and the Hadoken's (`+0x1c`).
    pub done: [bool; 3],
}

/// Per-session runtime state for Tier 1 core overhaul toggles (`#94` `O3` through `#97` `O6`),
/// snapshotted alongside [`crate::fighter::Program`] for deterministic replay takeover.
/// Fighter slots follow object RAM order. In Dramatic Battle that order is
/// Ally 1, boss, Ally 2. Latch pairs instead follow input order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CoreToggleState {
    /// Punch/kick pairs for P1, P2, and the Dramatic boss. Each flag indicates
    /// that the current non-zero latch byte came from a button release
    /// under [`CoreToggles::negative_edge_specials`].
    pub neg_edge_latch: [bool; 6],
    /// Buffered attack input per fighter under [`CoreToggles::recovery_input_buffer`].
    pub recovery_buffer: [Option<RecoveryBufferSlot>; 3],
    /// Per-fighter, per-tracker (`0 = Tatsumaki +0x16`,
    /// `1 = Shoryuken +0x19`, `2 = Hadoken +0x1c`) count of frames elapsed since
    /// releasing the final direction of a completed special command under
    /// [`CoreToggles::lenient_command_trackers`] (`#95`, `O4`).
    pub tracker_release_age: [[u8; 3]; 3],
    /// Flag per fighter tracking whether an active
    /// `Shoryuken` (`mv == 0x19`) or `Tatsumaki` (`mv == 0x1b`) has already connected
    /// (hit or blocked) during its current move execution under
    /// [`CoreToggles::balanced_roster`] (`#96` `O5`, `#97` `O6`, SP1 `#247`).
    pub dp_connected: [bool; 3],
    /// Count of consecutive frames each fighter has
    /// been in an active attack state (`+9 & 0x40 != 0`) for CPU reaction delay under
    /// [`CoreToggles::fair_cpu_and_sagat_scaling`] (`#97`, `O6`).
    pub human_attack_frames: [u8; 3],
    /// Remaining phase-1 (rising/early) airborne frames and vertical adjustment
    /// for an [`CoreToggles::anytime_jump_attacks`] air attack (`#95`, `O4`).
    pub air_adj_frames_1: [u8; 3],
    /// Remaining phase-1 vertical adjustment pixels per fighter (`#95`, `O4`).
    pub air_adj_rem_1: [i16; 3],
    /// Remaining phase-2 (descending) airborne frames and vertical adjustment
    /// for an [`CoreToggles::anytime_jump_attacks`] air attack (`#95`, `O4`).
    pub air_adj_frames_2: [u8; 3],
    /// Remaining phase-2 vertical adjustment pixels per fighter (`#95`, `O4`).
    pub air_adj_rem_2: [i16; 3],
    /// Hitstun, blockstun and the combo running under
    /// [`CoreToggles::true_combos`] (SP17, `#261`); default with it off.
    pub combo: crate::combo::ComboState,
    /// Each fighter's cancel window under [`CoreToggles::cancels`] (SP18,
    /// `#262`); default with it off.
    pub cancel: crate::combo::cancel::CancelState,
    /// Each fighter's throw tech window and post-wakeup throw invulnerability
    /// under [`CoreToggles::throws`] (SP19, `#263`); default with it off.
    pub throw: crate::combo::throw::ThrowState,
    /// Native three-fighter health and boss input state.
    pub dramatic: crate::dramatic::DramaticState,
    /// Native style picks, health, commands, and meters, by fight side.
    pub era: crate::era_fight::EraCombatState,
    /// Stage tune command a stand-in fight fell back to when Ryu's theme
    /// (`RYU_THEME`, `0x41`) was not in the playable extended tunes
    /// (SP15, `#267`).
    pub missing_ryu_theme: Option<u16>,
}
