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
    pub(super) fn for_participants(
        &mut self,
        setup: &FightSetup,
        bank: Option<&BackgroundBank>,
        participants: [ThemeId; 2],
    ) -> Option<MatchTheme> {
        let config = setup.config;
        let [_, opponent] = participants;
        match setup.kind {
            FightKind::Dramatic { .. } | FightKind::WorldTour => return Some(MatchTheme::single(opponent)),
            FightKind::Arcade if config.evil_ryu_playtest.is_some() => return Some(MatchTheme::single(opponent)),
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
        self.choose(stage, members, participants)
    }

    fn choose(&mut self, stage: &str, members: &'static [ThemeId], sides: [ThemeId; 2]) -> Option<MatchTheme> {
        let [p1, p2] = sides.map(|fighter| members.contains(&fighter).then_some(fighter));
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
    fn a_live_session_uses_associated_fighters_without_unlocking_them() {
        use super::super::{FrameInputs, GameSession};
        use crate::config::MatchConfig;
        use crate::front::cabinet::Players;
        use crate::front::fight::FightDips;
        use crate::roster::PlayMode;
        use sf_video::romset::load_assets;
        use sf_video::stage::pack_records;

        let roms = "../../tools/reference/local/roms/sf";
        if !std::path::Path::new(roms).is_dir() {
            eprintln!("skipping: local ROM set not present");
            return;
        }
        let assets = load_assets(roms, "../../tools/reference/rom-manifest.json").unwrap();
        let stage = include_str!("../../../../../art/stages/test-stage.json")
            .replace(r#""name": "test-stage""#, r#""name": "gouken""#);
        let bank = pack_records(&assets, &[&stage]).unwrap();
        let stage = bank.stage_index("gouken").unwrap();
        let mut config = MatchConfig::definitive();
        config.roster.mode = PlayMode::Versus;
        config.roster.stage = Some(StagePick::Bank(stage));
        let setup = FightSetup {
            kind: FightKind::Local,
            players: Players::Two,
            config,
            dsw1: FightDips::DSW1,
            dsw2: FightDips::default().dsw2(),
        };
        let mut session = GameSession::new(assets, config).with_background_bank(bank);
        // Keep using the actual released fighters on the gameplay side. The
        // fixture replaces their theme identities at the match launch seam.
        for (sides, id, phase) in [
            ([Gouken, Ryu], Gouken, 0),
            ([Akuma, Gouken], Gouken, 0),
            ([Gouken, Akuma], Akuma, 1),
            ([Akuma, Akuma], Akuma, 0),
            ([Gouken, Akuma], Gouken, 0),
        ] {
            session.launch_fight_with_themes(setup, Some(0), [None; 2], sides);
            let cue = (0..1000).find_map(|_| session.step(FrameInputs::default()).theme)
                .expect("the launched fight reaches its native theme latch");
            assert_eq!(cue.id, id);
            assert_eq!(session.fight().unwrap().selected_theme(), Some(MatchTheme { id, phase }));
        }
    }

    #[test]
    fn shared_stages_track_cycles_and_unordered_pairs_independently() {
        let mut rotation = ThemeRotation::default();
        let dojo = &[Gouken, Akuma, Goutetsu];
        let mut pick = |stage, members, sides| {
            let choice = rotation.choose(stage, members, sides).unwrap();
            (choice.id, choice.phase)
        };
        assert_eq!(pick("gouken", dojo, [Ryu, Ken]), (Gouken, 0));
        assert_eq!(pick("gouken", dojo, [Gouken, Gouken]), (Gouken, 0));
        assert_eq!(pick("gouken", dojo, [Gouken, Ryu]), (Gouken, 0));
        assert_eq!(pick("gouken", dojo, [Gouken, Akuma]), (Akuma, 0));
        assert_eq!(pick("gouken", dojo, [Akuma, Gouken]), (Gouken, 1));
        assert_eq!(pick("gouken", dojo, [Gouken, Goutetsu]), (Goutetsu, 0));
        assert_eq!(pick("other-dojo", dojo, [Akuma, Gouken]), (Gouken, 0));
        assert_eq!(pick("gouken", dojo, [Akuma, Gouken]), (Akuma, 0));
        assert_eq!(pick("gouken", dojo, [Ryu, Ken]), (Akuma, 1));
        assert_eq!(pick("gouken", &[Gouken, Goutetsu], [Ryu, Ken]), (Gouken, 0));
        assert_eq!(pick("gouken", dojo, [Ryu, Ken]), (Goutetsu, 2));
        assert_eq!(pick("gouken", dojo, [Ryu, Ken]), (Gouken, 0));
    }
}
