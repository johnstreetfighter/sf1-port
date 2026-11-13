//! Local Dramatic Battle setup and the three sequential fighter picks.

use sf_video::background_bank::StageListing;
use sf_video::busts::Bust;
use sf_video::compositor::{fallback_text, ink, RomFont};
use sf_video::render::Frame;

use super::draw::framed;
use super::select::SelectScreen;
use super::Transition;
use crate::config::MatchConfig;
use crate::dramatic::{DramaticRules, LifebarMode};
use crate::era::Revision;
use crate::roster::{MenuInput, StagePick, TournamentFighter};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rivalry {
    label: &'static str,
    ally1: TournamentFighter,
    ally2: TournamentFighter,
    boss: TournamentFighter,
    stage: TournamentFighter,
}

const RIVALRIES: [Rivalry; 6] = {
    use TournamentFighter::*;
    [
        Rivalry {
            label: "TWIN DRAGONS",
            ally1: Ryu,
            ally2: Ken,
            boss: Sagat,
            stage: Sagat,
        },
        Rivalry {
            label: "TEMPLE SCHISM",
            ally1: Retsu,
            ally2: Geki,
            boss: Gen,
            stage: Gen,
        },
        Rivalry {
            label: "AMERICAN HEAVYWEIGHTS",
            ally1: Joe,
            ally2: Mike,
            boss: Birdie,
            stage: Birdie,
        },
        Rivalry {
            label: "ROYAL GUARD",
            ally1: Birdie,
            ally2: Eagle,
            boss: Sagat,
            stage: Eagle,
        },
        Rivalry {
            label: "MUAY THAI COUP",
            ally1: Adon,
            ally2: Ryu,
            boss: Sagat,
            stage: Adon,
        },
        Rivalry {
            label: "FINAL TRIAL",
            ally1: Ryu,
            ally2: Sagat,
            boss: Gen,
            stage: Retsu,
        },
    ]
};

const ROWS: [&str; 5] = [
    "MATCHUP",
    "ERA STYLE",
    "LIFEBAR",
    "FRIENDLY FIRE",
    "START FIGHT",
];
const PICK_TITLES: [&str; 3] = ["SELECT ALLY 1", "SELECT ALLY 2", "SELECT BOSS"];
const NATIVE_STYLE: Revision = Revision::Sf1Plus;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DramaticMatch {
    pub ally1: TournamentFighter,
    pub boss: TournamentFighter,
    pub rules: DramaticRules,
}

/// The matchup and team rules selected before a local three-fighter fight.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DramaticSetupScreen {
    co_op: bool,
    matchup: usize,
    style: Revision,
    preset_styles: [Vec<Revision>; RIVALRIES.len()],
    lifebar: LifebarMode,
    friendly_fire: bool,
    row: usize,
}

impl DramaticSetupScreen {
    pub fn open(co_op: bool) -> Self {
        Self::for_config(co_op, &MatchConfig::definitive())
    }

    pub(super) fn for_config(co_op: bool, config: &MatchConfig) -> Self {
        let preset_styles = RIVALRIES.map(|rivalry| {
            DramaticRules::STYLE_REVISIONS.into_iter().filter(|&style| {
                [rivalry.ally1, rivalry.ally2, rivalry.boss].into_iter()
                    .all(|fighter| DramaticRules::style_available(config, fighter, style))
            }).collect()
        });
        Self {
            co_op,
            matchup: 0,
            style: NATIVE_STYLE,
            preset_styles,
            lifebar: LifebarMode::Shared,
            friendly_fire: false,
            row: 0,
        }
    }

    pub(super) fn from_rules(rules: DramaticRules, config: &MatchConfig) -> Self {
        let mut setup = Self::for_config(rules.co_op, config);
        setup.matchup = rules
            .rivalry
            .map(usize::from)
            .filter(|&index| index < RIVALRIES.len())
            .unwrap_or(RIVALRIES.len());
        setup.style = rules.styles[0];
        setup.lifebar = rules.lifebar;
        setup.friendly_fire = rules.friendly_fire;
        setup
    }

    pub fn name(&self) -> &'static str {
        "dramatic setup"
    }

    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        match input {
            MenuInput::Up | MenuInput::Down => {
                let step = if input == MenuInput::Down {
                    1
                } else {
                    ROWS.len() - 1
                };
                self.row = (self.row + step) % ROWS.len();
                Some(Transition::CursorMoved)
            }
            MenuInput::Left | MenuInput::Right => {
                match self.row {
                    0 => {
                        let choices = RIVALRIES.len() + 1;
                        let step = if input == MenuInput::Right {
                            1
                        } else {
                            choices - 1
                        };
                        self.matchup = (self.matchup + step) % choices;
                        if self.preset_styles.get(self.matchup).is_some_and(|styles| !styles.contains(&self.style)) {
                            self.style = NATIVE_STYLE;
                        }
                    }
                    1 => {
                        let styles = self.preset_styles.get(self.matchup).filter(|styles| styles.len() > 1)?;
                        let current = styles.iter().position(|&style| style == self.style).unwrap_or(0);
                        let step = if input == MenuInput::Right { 1 } else { styles.len() - 1 };
                        self.style = styles[(current + step) % styles.len()];
                    }
                    2 => {
                        self.lifebar = match self.lifebar {
                            LifebarMode::Shared => LifebarMode::Independent,
                            LifebarMode::Independent => LifebarMode::Shared,
                        }
                    }
                    3 => self.friendly_fire = !self.friendly_fire,
                    _ => return None,
                }
                Some(Transition::ValueChanged)
            }
            MenuInput::Confirm | MenuInput::Start if self.row == ROWS.len() - 1 => {
                Some(Transition::StartDramatic)
            }
            MenuInput::Back => Some(Transition::Back),
            _ => None,
        }
    }

    pub(super) fn preset(&self) -> Option<DramaticMatch> {
        let rivalry = RIVALRIES.get(self.matchup)?;
        Some(DramaticMatch {
            ally1: rivalry.ally1,
            boss: rivalry.boss,
            rules: self.rules(rivalry.ally2, rivalry.stage, [self.style; 3], Some(self.matchup as u8)),
        })
    }

    fn rules(
        &self,
        ally2: TournamentFighter,
        stage: TournamentFighter,
        styles: [Revision; 3],
        rivalry: Option<u8>,
    ) -> DramaticRules {
        DramaticRules {
            co_op: self.co_op,
            ally2,
            stage,
            lifebar: self.lifebar,
            friendly_fire: self.friendly_fire,
            styles,
            rivalry,
        }
    }

    pub fn render(&self, font: &RomFont<'_>, notice: Option<&str>) -> Frame {
        let header = if self.co_op {
            "LOCAL CO-OP DRAMATIC"
        } else {
            "DRAMATIC BATTLE"
        };
        let mut frame = framed(font, header, "");
        let argb = &mut frame.argb;
        let rivalry = RIVALRIES.get(self.matchup);
        if let Some(rivalry) = rivalry {
            let lineup = format!(
                "{} & {} VS {}",
                rivalry.ally1.name(),
                rivalry.ally2.name(),
                rivalry.boss.name()
            );
            font.text_centred(argb, 6, &lineup.to_ascii_uppercase(), ink::BLUE);
        }
        let values = [
            rivalry.map_or("CUSTOM 2V1", |rivalry| rivalry.label),
            if rivalry.is_some() { self.style.name() } else { "PER FIGHTER" },
            match self.lifebar {
                LifebarMode::Shared => "SHARED",
                LifebarMode::Independent => "INDEPENDENT",
            },
            if self.friendly_fire { "ON" } else { "OFF" },
            "",
        ];
        for (index, (label, value)) in ROWS.into_iter().zip(values).enumerate() {
            let row = 9 + 2 * index;
            let colour = if index == self.row {
                font.text(argb, 5, row, ">", ink::YELLOW);
                ink::YELLOW
            } else {
                ink::BLUE
            };
            font.text(argb, 7, row, label, colour);
            font.text(argb, 41 - value.len(), row, value, colour);
        }
        if let Some(notice) = notice {
            font.text_centred(argb, 21, notice, ink::YELLOW);
        }
        frame
    }
}

/// The normal single-cursor picker, advanced through Ally 1, Ally 2, and Boss.
#[derive(Clone)]
pub(super) struct DramaticPicks {
    setup: DramaticSetupScreen,
    selects: [SelectScreen; 3],
    chosen: [Option<(TournamentFighter, Revision)>; 3],
    role: usize,
    remote_ally: bool,
}

impl DramaticPicks {
    pub fn open(
        setup: DramaticSetupScreen,
        config: &MatchConfig,
        tables: &[TournamentFighter],
        remote_ally: Option<TournamentFighter>,
    ) -> Self {
        let selects = [0, 1, 1].map(|side| SelectScreen::dramatic(config, tables, side));
        Self {
            setup,
            selects,
            chosen: [None, remote_ally.map(|fighter| (fighter, NATIVE_STYLE)), None],
            role: 0,
            remote_ally: remote_ally.is_some(),
        }
    }

    pub fn select(&self) -> &SelectScreen {
        &self.selects[self.role]
    }

    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        self.selects[self.role].apply(input)
    }

    pub fn render(
        &self,
        font: &RomFont<'_>,
        busts: &[Option<Bust>; 12],
        frame_no: u32,
        notice: Option<&str>,
    ) -> Frame {
        let mut frame = self
            .select()
            .render_named(font, busts, PICK_TITLES[self.role], frame_no, Some((self.picker(), false)));
        if let Some(notice) = notice {
            font.text_centred(&mut frame.argb, 26, notice, ink::YELLOW);
        }
        frame
    }

    pub fn co_op(&self) -> bool {
        self.setup.co_op
    }

    pub fn picker(&self) -> usize {
        usize::from(self.setup.co_op && self.role == 1)
    }

    pub fn pick(&mut self, fighter: TournamentFighter) -> Option<DramaticMatch> {
        let select = &mut self.selects[self.role];
        let revision = select.locked_style(0).map_or(NATIVE_STYLE, |style| style.revision);
        if select.locked(0).is_some() {
            // Keep the chosen style with the fighter. Returning from a later
            // pick or the stage screen reopens this picker at the same era.
            select.apply(MenuInput::Back);
        }
        self.chosen[self.role] = Some((fighter, revision));
        if self.role == PICK_TITLES.len() - 1 {
            self.match_on(fighter)
        } else {
            self.role += if self.remote_ally { 2 } else { 1 };
            None
        }
    }

    pub fn back(&mut self) -> bool {
        if self.role == 0 {
            return false;
        }
        self.chosen[self.role] = None;
        self.role -= if self.remote_ally { 2 } else { 1 };
        self.chosen[self.role] = None;
        true
    }

    pub fn match_on(&self, stage: TournamentFighter) -> Option<DramaticMatch> {
        let (ally1, ally1_style) = self.chosen[0]?;
        let (ally2, ally2_style) = self.chosen[1]?;
        let (boss, boss_style) = self.chosen[2]?;
        Some(DramaticMatch {
            ally1,
            boss,
            rules: self.setup.rules(ally2, stage, [ally1_style, boss_style, ally2_style], None),
        })
    }
}

/// The online guest selects Ally 2 in a custom draft or accepts the preset ally.
#[derive(Clone)]
pub(super) struct DramaticGuest {
    select: SelectScreen,
    ally1: TournamentFighter,
    boss: TournamentFighter,
    rules: Option<DramaticRules>,
    stage: Option<StagePick>,
    fixed: Option<TournamentFighter>,
    locked: bool,
}

impl DramaticGuest {
    pub fn open(config: &MatchConfig, tables: &[TournamentFighter]) -> Self {
        let fixed = config
            .dramatic
            .filter(|rules| rules.rivalry.is_some())
            .map(|rules| rules.ally2);
        let mut select = if fixed.is_some() {
            SelectScreen::single(config, tables, 1)
        } else {
            SelectScreen::dramatic(config, tables, 1)
        };
        if let Some(rules) = config.dramatic {
            select.focus(rules.ally2);
            select = select.with_preferred_eras([rules.styles[2].family(); 2]);
        }
        Self {
            select,
            ally1: config.roster.p1,
            boss: config.roster.p2,
            rules: config.dramatic,
            stage: config.roster.stage,
            fixed,
            locked: false,
        }
    }

    pub fn fighter(&self) -> TournamentFighter {
        self.fixed.unwrap_or_else(|| self.select.fighter(0))
    }

    pub fn revision(&self) -> Revision {
        if self.fixed.is_some() {
            return self.rules.map_or(NATIVE_STYLE, |rules| rules.styles[2]);
        }
        self.select.locked_style(0).map_or(NATIVE_STYLE, |style| style.revision)
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    pub fn apply(&mut self, input: MenuInput) -> Option<Transition> {
        if input == MenuInput::Back && (self.locked || self.fixed.is_some()) {
            return Some(Transition::Back);
        }
        if self.locked
            || (self.fixed.is_some() && !matches!(input, MenuInput::Confirm | MenuInput::Start))
        {
            return None;
        }
        match self.select.apply(input) {
            Some(Transition::Fighter(fighter)) if fighter == self.fighter() => {
                self.locked = true;
                Some(Transition::Picked(fighter))
            }
            Some(Transition::Fighter(_)) => None,
            transition => transition,
        }
    }

    pub fn render(
        &self,
        font: &RomFont<'_>,
        busts: &[Option<Bust>; 12],
        stages: &[StageListing],
        frame_no: u32,
        notice: Option<&str>,
    ) -> Frame {
        let mut frame = self.select.render_named(font, busts, PICK_TITLES[1], frame_no, Some((1, self.locked)));
        if let Some(rules) = self.rules {
            let listed_stage = self.stage.and_then(|StagePick::Bank(index)| {
                stages.iter().find(|stage| stage.index == index)
            });
            let stage = listed_stage.map_or(rules.stage.name(), |stage| stage.owner.as_str());
            let lifebar = match rules.lifebar {
                LifebarMode::Shared => "SHARED",
                LifebarMode::Independent => "INDEPENDENT",
            };
            let fire = if rules.friendly_fire { "ON" } else { "OFF" };
            let details = [
                format!("ALLY 1 {} {}", self.ally1.name(), rules.styles[0].name()),
                format!("BOSS {} {}", self.boss.name(), rules.styles[1].name()),
                format!("STAGE {stage}"),
                format!("LIFEBAR {lifebar}"),
                format!("FRIENDLY FIRE {fire}"),
            ];
            for (row, detail) in details.iter().enumerate() {
                let text: String = detail.to_ascii_uppercase().chars().take(26).collect();
                fallback_text(&mut frame.argb, 52, 155 + 9 * row, &text, 0xffff_d438, 1);
            }
        }
        let status = if self.locked {
            "WAITING FOR HOST".into()
        } else if self.fixed.is_some() {
            format!("CONFIRM ALLY 2 {}", self.revision().name())
        } else {
            "CONFIRM ALLY 2".into()
        };
        font.text_centred(&mut frame.argb, 26, notice.unwrap_or(&status), ink::YELLOW);
        frame
    }
}

/// The four cabinet flags start the native flow. Thailand's stages enter
/// through Japan's flag and the Dramatic stage binding selects the fight.
pub(super) fn boot_flag(stage: TournamentFighter) -> u8 {
    use TournamentFighter::*;
    match stage {
        Ryu | Retsu | Geki | Adon | Sagat => 0,
        Lee | Gen => 1,
        Ken | Joe | Mike => 2,
        Birdie | Eagle => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rivalry_setup_cycles_through_six_native_matchups_and_custom() {
        use TournamentFighter::*;
        let mut setup = DramaticSetupScreen::open(false);
        let expected = [
            (Ryu, Ken, Sagat, Sagat),
            (Retsu, Geki, Gen, Gen),
            (Joe, Mike, Birdie, Birdie),
            (Birdie, Eagle, Sagat, Eagle),
            (Adon, Ryu, Sagat, Adon),
            (Ryu, Sagat, Gen, Retsu),
        ];
        for (index, want) in expected.into_iter().enumerate() {
            let matchup = setup.preset().expect("a rivalry");
            assert_eq!(
                (
                    matchup.ally1,
                    matchup.rules.ally2,
                    matchup.boss,
                    matchup.rules.stage
                ),
                want
            );
            assert_eq!(matchup.rules.rivalry, Some(index as u8));
            assert_eq!(matchup.rules.lifebar, LifebarMode::Shared);
            assert!(!matchup.rules.friendly_fire && !matchup.rules.co_op);
            setup.apply(MenuInput::Down);
            for style in DramaticRules::STYLE_REVISIONS {
                assert_eq!(setup.preset().unwrap().rules.styles, [style; 3], "rivalry {index}");
                assert_eq!(setup.apply(MenuInput::Right), Some(Transition::ValueChanged));
            }
            assert_eq!(setup.preset().unwrap().rules.styles, [NATIVE_STYLE; 3]);
            setup.apply(MenuInput::Up);
            setup.apply(MenuInput::Right);
        }
        assert!(setup.preset().is_none(), "custom follows the six rivalries");
        setup.apply(MenuInput::Right);
        assert_eq!(setup.preset().unwrap().rules.rivalry, Some(0));
    }

    #[test]
    fn guest_locks_the_preset_ally_and_can_leave_after_locking() {
        let mut config = MatchConfig::definitive();
        let mut setup = DramaticSetupScreen::for_config(true, &config);
        setup.apply(MenuInput::Down);
        setup.apply(MenuInput::Left);
        config.dramatic = Some(setup.preset().unwrap().rules);
        let mut guest = DramaticGuest::open(&config, &[]);
        assert_eq!(guest.fighter(), TournamentFighter::Ken);
        assert_eq!(guest.apply(MenuInput::Left), None);
        assert_eq!(
            guest.apply(MenuInput::Confirm),
            Some(Transition::Picked(TournamentFighter::Ken))
        );
        assert!(guest.locked());
        assert_eq!(guest.revision(), Revision::Sf6);
        assert_eq!(guest.apply(MenuInput::Right), None);
        assert_eq!(guest.apply(MenuInput::Back), Some(Transition::Back));
        assert_eq!(guest.fighter(), TournamentFighter::Ken);
    }
}
