use super::sound;
use super::{
    boot_flag, FightDips, FightKind, FightSetup, GameSession, MatchConfig, Players, Screen,
};
use crate::config::{WorldTourBonuses, WorldTourRules};
use crate::dramatic::{DramaticRules, LifebarMode};
use crate::era::{GaugeMode, Revision, StylePick};
use crate::era_fight::FULL_SUB_HP;
use crate::roster::{BanFormat, PlayMode, StagePick};
use crate::world_tour::native::{MissionKind, WorldTourNode};
use sf_video::rom_stages::rom_stage_name;

#[derive(Clone, Copy)]
pub(super) struct WorldTourRun {
    node: &'static WorldTourNode,
    wave: usize,
    pub(super) ending: bool,
}

impl GameSession {
    pub(super) fn world_tour_config(&self) -> MatchConfig {
        let mut config = self.config;
        if !config.era_select_enabled() {
            config.apply_preset(crate::config::RulesetPreset::DefinitiveOverhaul);
        }
        config.roster.full_cast = true;
        config.roster.ban = BanFormat::Unrestricted;
        config.roster.what_if_styles = self.config.roster.what_if_styles;
        config.roster.mode = PlayMode::Versus;
        config.fill_meters = false;
        config.dramatic = None;
        config.world_tour = None;
        config
    }

    pub(super) fn start_world_tour_node(&mut self) {
        if !matches!(self.stack.last(), Some(Screen::WorldTour(_))) {
            return;
        }
        let run = WorldTourRun {
            node: self.world_tour_profile.current_node(),
            wave: 0,
            ending: false,
        };
        self.start_world_tour_wave(run, None);
    }

    fn start_world_tour_wave(&mut self, run: WorldTourRun, starting_hp: Option<u16>) {
        self.play = None;
        let node = run.node;
        let profile = &self.world_tour_profile;
        let fighter = profile.fighter;
        let boss = node.opponents[run.wave];
        let ally = node.ally_for(fighter);
        let unavailable = std::iter::once(fighter)
            .chain(node.opponents.iter().copied())
            .chain(ally)
            .find(|&pick| !pick.is_shoto() && !self.player_tables().contains(&pick));
        if let Some(pick) = unavailable {
            self.notice = Some(format!(
                "{} NEEDS THE SF1+ ART PACK",
                pick.name().to_ascii_uppercase()
            ));
            return;
        }
        let name = rom_stage_name(node.stage.name());
        let Some(stage) = self
            .backgrounds
            .as_deref()
            .and_then(|bank| bank.stage_index(&name))
        else {
            self.notice = Some("WORLD TOUR STAGE IS UNAVAILABLE".into());
            return;
        };
        let mut config = self.world_tour_config();
        config.roster.p1 = fighter;
        config.roster.p2 = boss;
        config.roster.stage = Some(StagePick::Bank(stage));
        let loaned_style =
            matches!(node.mission, MissionKind::EraStyleMastery { .. }) || node.is_finale();
        let player_style = if loaned_style {
            if config.gauge_mode == GaugeMode::NoGauges {
                config.gauge_mode = GaugeMode::EraDefault;
            }
            node.player_style(profile.style)
        } else if profile
            .style
            .available_for(fighter, config.roster.what_if_styles)
        {
            profile.style
        } else {
            StylePick::for_preset(config.preset)
        };
        let opponent_style = node.opponent_style();
        config.world_tour = Some(WorldTourRules {
            mission: node.mission,
            bonuses: profile.rpg.then_some(WorldTourBonuses {
                attack: profile.attack,
                defense: profile.defense,
                speed: profile.speed,
                perk: profile.perk,
            }),
            starting_hp,
        });
        let (kind, players, second_style) = if let Some(ally2) = ally {
            let rules = DramaticRules {
                co_op: false,
                ally2,
                stage: node.stage,
                lifebar: LifebarMode::Independent,
                friendly_fire: false,
                styles: [
                    player_style.revision,
                    opponent_style.revision,
                    Revision::Sf1Plus,
                ],
                rivalry: None,
            };
            config.dramatic = Some(rules);
            config.p2_fair_cpu = false;
            (
                FightKind::Dramatic { co_op: false },
                Players::One,
                DramaticRules::style_pick(Revision::Sf1Plus),
            )
        } else {
            config.p2_fair_cpu = true;
            (FightKind::WorldTour, Players::Two, opponent_style)
        };
        let setup = FightSetup {
            kind,
            players,
            config,
            dsw1: FightDips::DSW1,
            dsw2: self.fight_dips().dsw2(),
        };
        self.notice = None;
        self.launch_fight(
            setup,
            Some(boot_flag(node.stage)),
            [Some(player_style), Some(second_style)],
        );
        if let Some(play) = &mut self.play {
            play.world_tour = Some(run);
        }
    }

    pub(super) fn finish_world_tour_match(&mut self, winner: Option<usize>) {
        let Some(play) = self.play.as_mut() else {
            return;
        };
        let Some(mut run) = play.world_tour else {
            return;
        };
        if winner != Some(0) {
            self.leave_world_tour_fight();
            self.notice = Some("MISSION FAILED".into());
            return;
        }
        if let MissionKind::SurvivalGauntlet { recovery_percent } = run.node.mission {
            if run.wave + 1 < run.node.opponents.len() {
                let remaining = play.fight.machine().era_fighters()[0].sub_hp;
                let recovery = FULL_SUB_HP * u16::from(recovery_percent) / 100;
                let starting_hp = remaining.saturating_add(recovery).min(FULL_SUB_HP);
                run.wave += 1;
                self.start_world_tour_wave(run, Some(starting_hp));
                return;
            }
        }
        self.world_tour_changed |= self.world_tour_profile.complete(run.node);
        if run.node.is_finale() {
            run.ending = true;
            play.world_tour = Some(run);
            play.fight.start_world_tour_ending();
        } else {
            self.leave_world_tour_fight();
            self.notice = Some("MISSION CLEARED".into());
        }
    }

    pub(super) fn leave_world_tour_fight(&mut self) {
        self.finish_replay_recording(false);
        self.play = None;
        self.sounds.extend([sound::STOP_ALL, sound::STOP_SAMPLES]);
    }
}

#[cfg(test)]
mod tests;
