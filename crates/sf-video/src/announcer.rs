//! The announcer's lines (spec #216, `EH8`, ADR 0011): what he can say, in
//! the order he calls them, and each line's file in the voice pack.
//!
//! The lines are asked for by the overhaul fight (`sf_engine::front::fight`)
//! and played by `sf_audio::announcer` on his own chip. He says the word on
//! screen, never a move's name, and a finished combo's count from
//! [`COMBO_MIN_HITS`] hits up. CPU sides are called as human ones are, so a
//! line carries no side.

use serde::{Deserialize, Serialize};

use crate::callouts::CalloutKind;

/// The player's local choice of tournament announcer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnouncerVoice {
    #[default]
    Ringside,
    Japanese,
}

impl AnnouncerVoice {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Ringside => "ringside",
            Self::Japanese => "japanese",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        [Self::Ringside, Self::Japanese].into_iter().find(|voice| voice.slug() == slug)
    }
}

/// The fewest hits a finished combo needs to be called ("Five hit combo!").
pub const COMBO_MIN_HITS: u16 = 5;

/// One line the announcer can say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AnnouncerLine {
    Super,
    Perfect,
    PerfectParry,
    Counter,
    PunishCounter,
    CrushCounter,
    AlphaCounter,
    Reversal,
    FirstAttack,
    Burnout,
    CustomCombo,
    /// "<hits> hit combo!", for a finished combo of [`COMBO_MIN_HITS`] or more.
    Combo { hits: u16 },
    Japan,
    Usa,
    England,
    China,
    Thailand,
    Fight,
    GameOver,
    TimeUp,
    Round1,
    Round2,
    Round3,
    Round4,
    Round5,
}

impl AnnouncerLine {
    /// The overhaul word lines (the combo counts aside).
    pub const WORDS: [AnnouncerLine; 11] = [
        AnnouncerLine::Super,
        AnnouncerLine::Perfect,
        AnnouncerLine::PerfectParry,
        AnnouncerLine::Counter,
        AnnouncerLine::PunishCounter,
        AnnouncerLine::CrushCounter,
        AnnouncerLine::AlphaCounter,
        AnnouncerLine::Reversal,
        AnnouncerLine::FirstAttack,
        AnnouncerLine::Burnout,
        AnnouncerLine::CustomCombo,
    ];

    /// Classic calls, in ROM latch order starting at `0x8f`.
    const CLASSIC: [(Self, &'static str); 13] = [
        (Self::Japan, "japan"),
        (Self::Usa, "usa"),
        (Self::England, "england"),
        (Self::China, "china"),
        (Self::Thailand, "thailand"),
        (Self::Fight, "fight"),
        (Self::GameOver, "game_over"),
        (Self::TimeUp, "time_up"),
        (Self::Round1, "round_1"),
        (Self::Round2, "round_2"),
        (Self::Round3, "round_3"),
        (Self::Round4, "round_4"),
        (Self::Round5, "round_5"),
    ];

    /// The 22 lines shipped in the Ringside pack.
    pub const OVERHAUL: [Self; 22] = {
        let mut lines = [Self::Super; 22];
        let mut i = 0;
        while i < Self::WORDS.len() {
            lines[i] = Self::WORDS[i];
            i += 1;
        }
        while i < lines.len() {
            lines[i] = Self::Combo { hits: COMBO_MIN_HITS + (i - Self::WORDS.len()) as u16 };
            i += 1;
        }
        lines
    };

    /// The complete 35-line Japanese pack.
    pub const ALL: [Self; 35] = {
        let mut lines = [Self::Super; 35];
        let mut i = 0;
        while i < Self::OVERHAUL.len() {
            lines[i] = Self::OVERHAUL[i];
            i += 1;
        }
        while i < lines.len() {
            lines[i] = Self::CLASSIC[i - Self::OVERHAUL.len()].0;
            i += 1;
        }
        lines
    };

    /// The classic announcer's line for this ROM latch.
    pub const fn from_rom_latch(latch: u8) -> Option<Self> {
        match latch {
            0x8f..=0x9b => Some(Self::CLASSIC[(latch - 0x8f) as usize].0),
            _ => None,
        }
    }

    /// The line for a call-out's moment: the word on screen. A move the
    /// player tried (`CalloutKind::Move`) has none: he never says a move's
    /// name.
    pub const fn for_callout(kind: CalloutKind) -> Option<AnnouncerLine> {
        Some(match kind {
            CalloutKind::Super { .. } => AnnouncerLine::Super,
            CalloutKind::Perfect => AnnouncerLine::Perfect,
            CalloutKind::PerfectParry => AnnouncerLine::PerfectParry,
            CalloutKind::Counter => AnnouncerLine::Counter,
            CalloutKind::PunishCounter => AnnouncerLine::PunishCounter,
            CalloutKind::CrushCounter => AnnouncerLine::CrushCounter,
            CalloutKind::AlphaCounter => AnnouncerLine::AlphaCounter,
            CalloutKind::Reversal => AnnouncerLine::Reversal,
            CalloutKind::FirstAttack => AnnouncerLine::FirstAttack,
            CalloutKind::Burnout => AnnouncerLine::Burnout,
            CalloutKind::CustomCombo => AnnouncerLine::CustomCombo,
            CalloutKind::Move { .. } | CalloutKind::Fix { .. } => return None,
        })
    }

    /// The line for a finished combo of `hits`: none under
    /// [`COMBO_MIN_HITS`].
    pub const fn for_combo(hits: u16) -> Option<AnnouncerLine> {
        if hits >= COMBO_MIN_HITS {
            Some(AnnouncerLine::Combo { hits })
        } else {
            None
        }
    }

    /// Where the line stands in his order, higher first: classic calls, super and PERFECT;
    /// Perfect Parry; the counters; reversal; first attack; Burnout and
    /// Custom Combo; the combo count. A higher line cuts off a lower one;
    /// an equal or lower one waits.
    pub const fn priority(self) -> u8 {
        match self {
            AnnouncerLine::Super
            | AnnouncerLine::Perfect
            | AnnouncerLine::Japan
            | AnnouncerLine::Usa
            | AnnouncerLine::England
            | AnnouncerLine::China
            | AnnouncerLine::Thailand
            | AnnouncerLine::Fight
            | AnnouncerLine::GameOver
            | AnnouncerLine::TimeUp
            | AnnouncerLine::Round1
            | AnnouncerLine::Round2
            | AnnouncerLine::Round3
            | AnnouncerLine::Round4
            | AnnouncerLine::Round5 => 6,
            AnnouncerLine::PerfectParry => 5,
            AnnouncerLine::Counter
            | AnnouncerLine::PunishCounter
            | AnnouncerLine::CrushCounter
            | AnnouncerLine::AlphaCounter => 4,
            AnnouncerLine::Reversal => 3,
            AnnouncerLine::FirstAttack => 2,
            AnnouncerLine::Burnout | AnnouncerLine::CustomCombo => 1,
            AnnouncerLine::Combo { .. } => 0,
        }
    }

    /// The line's ID, which is also its voice-pack file stem.
    pub fn id(self) -> String {
        self.file_stem()
    }

    /// The line's file in the voice pack, without its extension:
    /// `super`, `punish_counter`, `combo_5`.
    pub fn file_stem(self) -> String {
        let word = match self {
            AnnouncerLine::Super => "super",
            AnnouncerLine::Perfect => "perfect",
            AnnouncerLine::PerfectParry => "perfect_parry",
            AnnouncerLine::Counter => "counter",
            AnnouncerLine::PunishCounter => "punish_counter",
            AnnouncerLine::CrushCounter => "crush_counter",
            AnnouncerLine::AlphaCounter => "alpha_counter",
            AnnouncerLine::Reversal => "reversal",
            AnnouncerLine::FirstAttack => "first_attack",
            AnnouncerLine::Burnout => "burnout",
            AnnouncerLine::CustomCombo => "custom_combo",
            AnnouncerLine::Combo { hits } => return format!("combo_{hits}"),
            classic => Self::CLASSIC
                .iter()
                .find_map(|&(line, stem)| (line == classic).then_some(stem))
                .expect("a classic announcer line"),
        };
        word.to_string()
    }

    /// The line a voice-pack file stem names, if any.
    pub fn from_file_stem(stem: &str) -> Option<AnnouncerLine> {
        if let Some(hits) = stem.strip_prefix("combo_") {
            // `combo_05` is not `combo_5`: one spelling per line.
            let hits: u16 = hits.parse().ok().filter(|_| !hits.starts_with('0'))?;
            return AnnouncerLine::for_combo(hits);
        }
        Self::WORDS.into_iter().find(|line| line.file_stem() == stem).or_else(|| {
            Self::CLASSIC.iter().find_map(|&(line, id)| (id == stem).then_some(line))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_call_out_has_its_line_and_the_order_is_the_specs() {
        for kind in CalloutKind::all("SHINKU HADOKEN") {
            let line = AnnouncerLine::for_callout(kind).expect("a line");
            assert_eq!(AnnouncerLine::from_file_stem(&line.file_stem()), Some(line), "{kind:?}");
        }
        assert_eq!(AnnouncerLine::for_callout(CalloutKind::Move { word: "HADOKEN!" }), None, "never a move's name");
        let order = |l: AnnouncerLine| l.priority();
        assert_eq!(order(AnnouncerLine::Super), order(AnnouncerLine::Perfect));
        assert!(order(AnnouncerLine::Perfect) > order(AnnouncerLine::PerfectParry));
        assert!(order(AnnouncerLine::PerfectParry) > order(AnnouncerLine::Counter));
        for c in [AnnouncerLine::PunishCounter, AnnouncerLine::CrushCounter, AnnouncerLine::AlphaCounter] {
            assert_eq!(order(c), order(AnnouncerLine::Counter));
        }
        assert!(order(AnnouncerLine::Counter) > order(AnnouncerLine::Reversal));
        assert!(order(AnnouncerLine::Reversal) > order(AnnouncerLine::FirstAttack));
        assert!(order(AnnouncerLine::FirstAttack) > order(AnnouncerLine::Burnout));
        assert_eq!(order(AnnouncerLine::Burnout), order(AnnouncerLine::CustomCombo));
        assert!(order(AnnouncerLine::CustomCombo) > order(AnnouncerLine::Combo { hits: 9 }));
    }

    #[test]
    fn only_the_thirteen_classic_announcements_map_from_the_rom_latch() {
        let stems = [
            "japan", "usa", "england", "china", "thailand", "fight", "game_over",
            "time_up", "round_1", "round_2", "round_3", "round_4", "round_5",
        ];
        for latch in u8::MIN..=u8::MAX {
            let line = AnnouncerLine::from_rom_latch(latch);
            if (0x8f..=0x9b).contains(&latch) {
                let line = line.expect("classic announcer line");
                assert_eq!(line.id(), stems[usize::from(latch - 0x8f)]);
                assert_eq!(line.priority(), 6);
            } else {
                assert_eq!(line, None, "latch {latch:#04x}");
            }
        }
        assert_eq!(AnnouncerLine::OVERHAUL.len(), 22);
        let mut unique = std::collections::BTreeSet::new();
        for line in AnnouncerLine::ALL {
            let stem = line.file_stem();
            assert_eq!(AnnouncerLine::from_file_stem(&stem), Some(line));
            assert!(unique.insert(stem));
        }
        assert_eq!(unique.len(), 35);
    }

    #[test]
    fn combos_are_called_from_five_hits() {
        assert_eq!(AnnouncerLine::for_combo(4), None);
        assert_eq!(AnnouncerLine::for_combo(5), Some(AnnouncerLine::Combo { hits: 5 }));
        assert_eq!(AnnouncerLine::from_file_stem("combo_12"), Some(AnnouncerLine::Combo { hits: 12 }));
        for stem in ["combo_4", "combo_05", "combo_", "combo_x", "fireball", "SUPER"] {
            assert_eq!(AnnouncerLine::from_file_stem(stem), None, "{stem}");
        }
    }
}
