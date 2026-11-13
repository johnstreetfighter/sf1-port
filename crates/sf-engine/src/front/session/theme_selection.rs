use std::collections::BTreeMap;

use crate::front::fight::{FightKind, FightSetup, MatchTheme};
use crate::roster::StagePick;
use crate::theme::{stage_members, ThemeId};
use sf_video::background_bank::BackgroundBank;

#[derive(Clone, Default)]
pub(super) struct ThemeRotation {
    stage_cycles: BTreeMap<(String, &'static [ThemeId]), u8>,
    pair_phases: BTreeMap<(String, [ThemeId; 2]), PairPhase>,
}

#[derive(Clone, Copy)]
struct PairPhase {
    order: [ThemeId; 2],
    next: bool,
}

impl ThemeRotation {
    pub(super) fn for_match(&mut self, setup: &FightSetup, bank: Option<&BackgroundBank>) -> Option<MatchTheme> {
        let config = setup.config;
        let [p1, p2] = [config.roster.p1, config.roster.p2]
            .map(|fighter| ThemeId::from_fighter_slug(&fighter.name().to_ascii_lowercase()));
        match setup.kind {
            FightKind::Dramatic { .. } | FightKind::WorldTour => return p2.map(MatchTheme::single),
            FightKind::Arcade if config.evil_ryu_playtest.is_some() => return p2.map(MatchTheme::single),
            FightKind::Arcade => return None, // The native Arcade ladder chooses its opponent per match.
            _ => {}
        }
        let StagePick::Bank(index) = config.roster.stage?;
        let bank = bank?;
        let stage = bank.stage_names().nth(usize::from(index))?;
        let members = stage_members(stage);
        if members.is_empty() {
            let command = u8::try_from(bank.stage_record(index)?.theme).ok()?;
            return ThemeId::from_command(command).map(MatchTheme::single);
        }
        self.choose(stage, members, [p1, p2])
    }

    fn choose(&mut self, stage: &str, members: &'static [ThemeId], sides: [Option<ThemeId>; 2]) -> Option<MatchTheme> {
        let [p1, p2] = sides.map(|fighter| fighter.filter(|id| members.contains(id)));
        match (p1, p2) {
            (Some(p1), Some(p2)) if p1 != p2 => {
                let mut pair = [p1, p2];
                pair.sort_unstable();
                let rotation = self.pair_phases.entry((stage.to_owned(), pair))
                    .or_insert(PairPhase { order: [p2, p1], next: false });
                let phase = u8::from(rotation.next);
                rotation.next = !rotation.next;
                Some(MatchTheme { id: rotation.order[usize::from(phase)], phase })
            }
            (Some(id), _) | (_, Some(id)) => Some(MatchTheme::single(id)),
            (None, None) if members.len() <= 1 => members.first().copied().map(MatchTheme::single),
            (None, None) => {
                let next = self.stage_cycles.entry((stage.to_owned(), members)).or_default();
                let id = *members.get(usize::from(*next))?;
                let phase = *next;
                *next = (phase + 1) % members.len() as u8;
                Some(MatchTheme { id, phase })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeId::{Akuma, Goutetsu, Gouken, Ken, Ryu};

    #[test]
    fn shared_stages_track_cycles_and_unordered_pairs_independently() {
        let mut rotation = ThemeRotation::default();
        let dojo = &[Gouken, Akuma, Goutetsu];
        let mut pick = |stage, members, sides| {
            let choice = rotation.choose(stage, members, sides).unwrap();
            (choice.id, choice.phase)
        };
        assert_eq!(pick("gouken", dojo, [Some(Ryu), Some(Ken)]), (Gouken, 0));
        assert_eq!(pick("gouken", dojo, [Some(Gouken), Some(Gouken)]), (Gouken, 0));
        assert_eq!(pick("gouken", dojo, [Some(Gouken), Some(Ryu)]), (Gouken, 0));
        assert_eq!(pick("gouken", dojo, [Some(Gouken), Some(Akuma)]), (Akuma, 0));
        assert_eq!(pick("gouken", dojo, [Some(Akuma), Some(Gouken)]), (Gouken, 1));
        assert_eq!(pick("gouken", dojo, [Some(Gouken), Some(Goutetsu)]), (Goutetsu, 0));
        assert_eq!(pick("other-dojo", dojo, [Some(Akuma), Some(Gouken)]), (Gouken, 0));
        assert_eq!(pick("gouken", dojo, [Some(Akuma), Some(Gouken)]), (Akuma, 0));
        assert_eq!(pick("gouken", dojo, [Some(Ryu), Some(Ken)]), (Akuma, 1));
        assert_eq!(pick("gouken", &[Gouken, Goutetsu], [Some(Ryu), Some(Ken)]), (Gouken, 0));
        assert_eq!(pick("gouken", dojo, [Some(Ryu), Some(Ken)]), (Goutetsu, 2));
        assert_eq!(pick("gouken", dojo, [Some(Ryu), Some(Ken)]), (Gouken, 0));
    }
}
