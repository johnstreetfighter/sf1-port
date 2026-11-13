//! Match the displayed gauge rows with the fighter's meter rules.

use sf_video::era_hud::{Fill, GaugeKind, GaugePresentation, RowPresentation};

use super::{EraFighterState, EvilRyuKit, GaugeMode, Loadout, Revision, STOCK_AT, sf4, sf5, sf6};

impl EraFighterState {
    pub(crate) fn gauge_presentation(&self, show_trigger_timer: bool) -> Option<GaugePresentation> {
        if self.super_capacity == 0 {
            return None;
        }

        let mut main = self.super_presentation();
        let mut second = None;
        if matches!(
            self.evil_ryu_kit,
            EvilRyuKit::EvilAlpha2 { custom_left: 1.. }
        ) {
            second = Some(main);
            main = RowPresentation::new(GaugeKind::AlphaCustom);
        } else if !self.unified_gauge {
            if self.is_sf6() {
                second = Some(main);
                let drive = self.sf6_drive_points().unwrap_or_default();
                let ready = if self.sf6_can_pay_drive(sf6::DRIVE_PARRY_START_COST) {
                    ready_stocks(
                        drive,
                        sf6::FULL_DRIVE,
                        STOCK_AT,
                        Some(self.meter_cost(sf6::DRIVE_PARRY_START_COST)),
                    )
                } else {
                    Fill::EMPTY
                };
                main = RowPresentation::new(GaugeKind::Sf6Drive).with_ready(ready);
            } else if self.is_sf4() {
                let ready = if self.sf4_ultra_ready() {
                    Fill::of(self.sf4.revenge, sf4::REVENGE_MAX)
                } else {
                    Fill::EMPTY
                };
                second = Some(RowPresentation::new(GaugeKind::Sf4Revenge).with_ready(ready));
            } else if let Some((meter, timer, _)) = self.sf5_practice_points() {
                let mode = GaugeMode::EraDefault;
                let kind = if show_trigger_timer && timer != 0 {
                    GaugeKind::Sf5VTrigger
                } else {
                    GaugeKind::Sf5V
                };
                let ready = if self.can_spend_v(mode, STOCK_AT) {
                    ready_stocks(
                        meter.into(),
                        self.v_trigger_cost(mode).into(),
                        STOCK_AT,
                        Some(self.meter_cost(STOCK_AT)),
                    )
                } else {
                    Fill::EMPTY
                };
                second = Some(RowPresentation::new(kind).with_ready(ready));
            } else if self.guard_power().is_some() {
                let kind = match self.style.loadout {
                    Loadout::XIsm => GaugeKind::Alpha3GuardX,
                    Loadout::VIsm => GaugeKind::Alpha3GuardV,
                    _ => GaugeKind::Alpha3GuardA,
                };
                second = Some(RowPresentation::new(kind));
            }
        }
        if self.sub_hp == 0 {
            main.ready = Fill::EMPTY;
            if let Some(row) = &mut second {
                row.ready = Fill::EMPTY;
            }
        }
        Some(GaugePresentation { main, second })
    }

    fn super_presentation(&self) -> RowPresentation {
        let kind = if self.unified_gauge {
            GaugeKind::Unified
        } else {
            match self.style.revision {
                Revision::Sfz1 => GaugeKind::Alpha1Super,
                Revision::Sfz2 | Revision::Sfz2A => GaugeKind::Alpha2Super,
                Revision::Sfz3 => match self.style.loadout {
                    Loadout::VIsm => GaugeKind::Alpha3V,
                    Loadout::XIsm => GaugeKind::Alpha3X,
                    _ => GaugeKind::Alpha3A,
                },
                Revision::Sf3Ng => GaugeKind::Sf3NewGeneration,
                Revision::Sf3TwoI => GaugeKind::Sf3SecondImpact,
                Revision::Sf3ThreeS => GaugeKind::Sf3ThirdStrike,
                Revision::Sf4 | Revision::Ssf4 | Revision::Ae2012 | Revision::Usf4 => {
                    GaugeKind::Sf4Super
                }
                Revision::Sf5 | Revision::Sf5Ae | Revision::Sf5Ce => GaugeKind::Sf5Critical,
                Revision::Sf6 => GaugeKind::Sf6SuperArt,
                _ => GaugeKind::Sf2Super,
            }
        };

        let cost = self.super_ready_cost().map(|cost| self.meter_cost(cost));
        let ready = if self.sf3_gauge().is_some() {
            // The level digit counts completed stocks. The bar shows the next
            // stock's progress. Readiness normally lights the digit, but an
            // ISM Plus discount can make the first Art usable before level 1.
            let stock = self.sf3_stock_cost().unwrap_or(STOCK_AT);
            ready_stocks(self.super_meter.into(), stock.into(), stock, cost)
        } else {
            ready_stocks(
                self.super_meter.into(),
                self.super_capacity.into(),
                STOCK_AT,
                cost,
            )
        };
        RowPresentation::new(kind).with_ready(ready)
    }

    // The least meter an available action needs, before the World Tour
    // discount. Input windows and move recovery are deliberately separate.
    fn super_ready_cost(&self) -> Option<u8> {
        if self.st_new()
            || (self.what_if_active()
                && self.style.revision == Revision::St
                && self.style.loadout == Loadout::Standard)
        {
            Some(self.super_capacity)
        } else if self.is_alpha() {
            match self.style.loadout {
                _ if self.evil_ryu_alpha2() && self.custom_combo_active() => None,
                Loadout::XIsm => self.alpha_super_cost(3),
                Loadout::VIsm if !self.unified_gauge && self.custom_combo_active() => None,
                _ => Some(STOCK_AT), // Super, Custom Combo, or Alpha Counter
            }
        } else if self.is_sf3() {
            let art = self.sf3_super_art_cost();
            if self.unified_gauge && self.sf3_has_ex() {
                Some(self.sf3_ex_cost())
            } else if self.unified_gauge {
                self.what_if_ex_super_cost().into_iter().chain(art).min()
            } else {
                art
            }
        } else if self.is_sf4() {
            if self.sf4_has_single_stock_move() || self.what_if_ex_super_cost().is_some() {
                Some(STOCK_AT)
            } else {
                Some(self.sf4_focus_cancel_cost())
            }
        } else if sf5::available(self.style, self.fighter) {
            if self.sf5_has_ex()
                || self.what_if_ex_super_cost().is_some()
                || (self.unified_gauge && !self.sf5.warm())
            {
                Some(STOCK_AT)
            } else {
                Some(3 * STOCK_AT)
            }
        } else if self.is_sf6() {
            Some(STOCK_AT)
        } else {
            None
        }
    }
}

fn ready_stocks(amount: u16, capacity: u16, stock: u8, cost: Option<u8>) -> Fill {
    let Some(cost) = cost
        .map(u16::from)
        .filter(|cost| *cost != 0 && amount >= *cost)
    else {
        return Fill::EMPTY;
    };
    let stock = u16::from(stock);
    let completed = amount / stock * stock;
    // Partially charged stocks ordinarily stay steady. Below the first stock,
    // show the portion that can already pay for parry or a discounted move.
    Fill::of(completed.max(cost).min(capacity), capacity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::era::StylePick;
    use crate::era_fight::FULL_SUB_HP;
    use crate::roster::TournamentFighter::Ryu;
    use crate::world_tour::native::Perk;

    fn fighter(revision: Revision, loadout: Loadout, mode: GaugeMode) -> EraFighterState {
        let mut state = EraFighterState {
            style: StylePick::for_revision(revision, loadout),
            fighter: Some(Ryu),
            sub_hp: FULL_SUB_HP,
            max_sub_hp: FULL_SUB_HP,
            ..EraFighterState::default()
        };
        state.set_gauge_mode(mode);
        state
    }

    #[test]
    fn alpha_ism_rows_keep_guard_passive_and_show_discounted_custom_readiness() {
        let mode = GaugeMode::EraDefault;
        let mut state = fighter(Revision::Sfz3, Loadout::VIsm, mode);
        state.tour.perk = Some(Perk::IsmPlus);
        let cost = state.meter_cost(STOCK_AT);
        state.set_super_meter(Some(cost - 1));
        assert_eq!(
            state.gauge_presentation(true).unwrap().main.ready,
            Fill::EMPTY
        );
        state.set_super_meter(Some(cost));
        let view = state.gauge_presentation(true).unwrap();
        assert_eq!(view.main.kind, GaugeKind::Alpha3V);
        assert_eq!(
            view.main.ready,
            Fill::of(cost.into(), (2 * STOCK_AT).into())
        );
        assert_eq!(
            view.second.unwrap(),
            RowPresentation::new(GaugeKind::Alpha3GuardV)
        );

        state.custom_frames = 3;
        assert_eq!(
            state.gauge_presentation(true).unwrap().main.ready,
            Fill::EMPTY
        );
        assert!(matches!(
            state.gauge().unwrap().main,
            super::super::GaugeRow::Stock {
                state: super::super::GaugeState::Draining,
                ..
            }
        ));

        let mut unified = fighter(Revision::Sfz3, Loadout::VIsm, GaugeMode::Unified);
        unified.set_super_meter(Some(STOCK_AT));
        let view = unified.gauge_presentation(true).unwrap();
        assert_eq!(view.main.kind, GaugeKind::Unified);
        assert_eq!(
            view.main.ready,
            Fill::of(STOCK_AT.into(), (3 * STOCK_AT).into())
        );
        assert!(view.second.is_none());
    }

    #[test]
    fn native_third_strike_ready_digit_uses_the_selected_art_price() {
        let mut state = fighter(Revision::Sf3ThreeS, Loadout::Art2, GaugeMode::EraDefault);
        let cost = state.sf3_super_art_cost().unwrap();
        state.set_super_meter(Some(cost - 1));
        assert_eq!(
            state.gauge_presentation(true).unwrap().main.ready,
            Fill::EMPTY
        );
        state.set_super_meter(Some(cost));
        let view = state.gauge_presentation(true).unwrap();
        assert_eq!(view.main.kind, GaugeKind::Sf3ThirdStrike);
        assert_eq!(view.main.ready.px(100), 100);

        state.tour.perk = Some(Perk::IsmPlus);
        let discount = state.meter_cost(cost);
        state.set_super_meter(Some(discount));
        assert_eq!(
            state.gauge_presentation(true).unwrap().main.ready,
            Fill::of(discount.into(), cost.into()),
        );
    }

    #[test]
    fn sf5_trigger_timer_has_its_own_label_and_cannot_be_spent_as_shared_reserve() {
        let mut state = fighter(
            Revision::Sf5Ce,
            Loadout::Skill1Trigger1,
            GaugeMode::EraDefault,
        );
        state.set_sf5_practice_points((STOCK_AT, 30, 40));
        let individual = state.gauge_presentation(true).unwrap().second.unwrap();
        let team = state.gauge_presentation(false).unwrap().second.unwrap();
        assert_eq!(individual, RowPresentation::new(GaugeKind::Sf5VTrigger));
        assert_eq!(team, RowPresentation::new(GaugeKind::Sf5V));

        state.set_sf5_practice_points((STOCK_AT, 0, 0));
        let team = state.gauge_presentation(false).unwrap().second.unwrap();
        assert_eq!(team.kind, GaugeKind::Sf5V);
        assert_eq!(team.ready.px(96), 48);
        state.sub_hp = 0;
        assert_eq!(
            state.gauge_presentation(false).unwrap().second.unwrap(),
            RowPresentation::new(GaugeKind::Sf5V),
        );
    }

    #[test]
    fn native_drive_marks_only_discrete_owned_spends_and_stops_pulsing_in_burnout() {
        use crate::config::{MatchConfig, RulesetPreset};
        use crate::fighter::{P1_STRUCT, Program};
        use crate::machine::MachineState;

        let mut program = Program::power_on(vec![]);
        program.match_config = MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul);
        let mut machine = MachineState::new(vec![], vec![], vec![], 0, 0, 0, false, 0xff, 0);
        let mut bus = program.bus(&mut machine);
        bus.wb(crate::fighter::GAME_MODE, 2);
        bus.wb(0xff8003, 1);
        let mut state = fighter(Revision::Sf6, Loadout::Standard, GaugeMode::EraDefault);
        let start_cost = u16::from(sf6::DRIVE_PARRY_START_COST);
        state.set_sf6_practice_drive(Some(start_cost - 1));
        assert_eq!(
            state.gauge_presentation(true).unwrap().main.ready,
            Fill::EMPTY
        );
        state.set_sf6_practice_drive(Some(start_cost));
        let view = state.gauge_presentation(true).unwrap();
        assert_eq!(view.main.kind, GaugeKind::Sf6Drive);
        assert_eq!(view.main.ready, Fill::of(start_cost, sf6::FULL_DRIVE));
        state.restore_sf6_practice_drive(start_cost, true);
        assert_eq!(
            state.gauge_presentation(true).unwrap().main.ready,
            Fill::EMPTY
        );

        state.set_sf6_practice_drive(None);
        bus.toggle_state.era.fighters[0] = state;
        bus.sf6_drain_drive(P1_STRUCT, STOCK_AT);
        assert!(bus.sf6_spend(P1_STRUCT, 1));
        assert_eq!(bus.toggle_state.era.gauge_drive_spends, [false; 3]);
        assert!(bus.sf6_spend(P1_STRUCT, sf6::DRIVE_PARRY_START_COST));
        assert_eq!(
            bus.toggle_state.era.gauge_drive_spends,
            [true, false, false]
        );
    }
}
