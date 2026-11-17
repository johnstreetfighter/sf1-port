//! Presentation history for the real Fight HUD. Meter amounts come from the
//! machine before and after a step, after any external Training refill.

use sf_video::era_hud::{
    Fill, GaugeCharge, GaugeKind, GaugePresentation, GaugeRow, GaugeState, RowPresentation,
};

const DRAIN_FRAMES: u8 = 12;
const FLASH_CYCLE: u8 = 33;
const FLASH_ON: u8 = 11;
const PULSE_CYCLE: u8 = 84;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Corner {
    pub gauge: sf_video::era_hud::EraGauge,
    pub presentation: GaugePresentation,
}

impl Corner {
    pub(super) fn share_readiness(&mut self, partner: GaugePresentation) {
        fn merge(row: &mut RowPresentation, other: RowPresentation) {
            if row.kind == other.kind && more(other.ready, row.ready) {
                row.ready = other.ready;
            }
        }
        merge(&mut self.presentation.main, partner.main);
        if let (Some(row), Some(other)) = (&mut self.presentation.second, partner.second) {
            merge(row, other);
        }
    }

    fn rows(self) -> [Option<Row>; 2] {
        [
            Some(Row {
                gauge: self.gauge.main,
                presentation: self.presentation.main,
            }),
            self.gauge
                .second
                .zip(self.presentation.second)
                .map(|(gauge, presentation)| Row {
                    gauge,
                    presentation,
                }),
        ]
    }
}

#[derive(Clone, Copy)]
struct Row {
    gauge: GaugeRow,
    presentation: RowPresentation,
}

impl Row {
    fn amount(self) -> Fill {
        match self.gauge {
            GaugeRow::Stock {
                cells, full, part, ..
            } => {
                let scale = u16::from(u8::MAX);
                let num = u16::from(full) * scale + part.px(usize::from(scale)) as u16;
                Fill::of(num, u16::from(cells) * scale)
            }
            GaugeRow::Level { fill, .. } | GaugeRow::Bar { fill, .. } => fill,
        }
    }

    fn full(self) -> bool {
        // SFIII shows the fraction toward the next stock, and its own state
        // tells whether the final stock is complete.
        match self.gauge {
            GaugeRow::Level { state, .. } => state == GaugeState::Full,
            _ => {
                let amount = self.amount();
                amount.den != 0 && amount.num >= amount.den
            }
        }
    }

    fn count(self) -> Option<u8> {
        match self.gauge {
            GaugeRow::Level { level, .. } => Some(level),
            _ => None,
        }
    }

    fn newly_ready(self, before: Self) -> Option<GaugeCharge> {
        let (from, to) = match self.gauge {
            GaugeRow::Level { .. } if self.count() > before.count() => (Fill::EMPTY, Fill::FULL),
            // A continuous gauge's ready span can grow every frame after
            // crossing the threshold. Only its first usable frame is a cue.
            GaugeRow::Bar { .. } if before.presentation.ready.num != 0 => return None,
            _ => (before.presentation.ready, self.presentation.ready),
        };
        (self.presentation.ready.num != 0 && more(to, from)).then_some(GaugeCharge {
            from,
            to,
            age: 0,
        })
    }

    fn can_animate(self) -> bool {
        !matches!(
            self.gauge,
            GaugeRow::Stock {
                state: GaugeState::Draining,
                ..
            } | GaugeRow::Level {
                state: GaugeState::Draining,
                ..
            } | GaugeRow::Bar {
                state: GaugeState::Draining,
                ..
            }
        )
    }

    fn same_shape(self, other: Self) -> bool {
        match (self.gauge, other.gauge) {
            (GaugeRow::Stock { cells: a, .. }, GaugeRow::Stock { cells: b, .. }) => a == b,
            (GaugeRow::Level { length: a, .. }, GaugeRow::Level { length: b, .. })
            | (GaugeRow::Bar { length: a, .. }, GaugeRow::Bar { length: b, .. }) => a == b,
            _ => false,
        }
    }
}

fn more(a: Fill, b: Fill) -> bool {
    a.den != 0
        && u32::from(a.num.min(a.den)) * u32::from(b.den.max(1))
            > u32::from(b.num.min(b.den)) * u32::from(a.den)
}

/// Kept with the fight's rollback snapshot so a replay produces the same
/// presentation. It never affects the machine or its gameplay checksum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct GaugeEffects {
    corners: [[RowEffects; 2]; 3],
}

#[derive(Clone, Copy)]
pub(super) struct Spends {
    pub drive: [bool; 3],
    pub v_trigger: [bool; 3],
}

impl GaugeEffects {
    pub(super) fn clear_side(&mut self, side: usize) {
        self.corners[side] = [RowEffects::default(); 2];
    }

    pub(super) fn step(
        &mut self,
        before: [Option<Corner>; 3],
        after: [Option<Corner>; 3],
        spends: Spends,
    ) {
        for (side, ((fx, before), after)) in
            self.corners.iter_mut().zip(before).zip(after).enumerate()
        {
            let before = before.map_or([None; 2], Corner::rows);
            let after = after.map_or([None; 2], Corner::rows);
            for ((effect, before), after) in fx.iter_mut().zip(before).zip(after) {
                effect.step(before, after, spends.drive[side], spends.v_trigger[side]);
            }
        }
    }

    pub(super) fn present(&self, side: usize, corner: Corner) -> GaugePresentation {
        let [main, second] = corner.rows();
        let [main_fx, second_fx] = self.corners[side];
        GaugePresentation {
            main: main_fx.present(main.expect("gauge has a main row")),
            second: second.map(|row| second_fx.present(row)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RowEffects {
    flash_phase: u8,
    pulse_phase: u8,
    charge: Option<GaugeCharge>,
    drain: Option<Drain>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Drain {
    from: Fill,
    to: Fill,
    age: u8,
}

impl Drain {
    const DIGIT_ONLY: Fill = Fill::of(1, 4096);

    fn new(from: Fill, to: Fill) -> Self {
        Self { from, to, age: 0 }
    }

    fn edge(self) -> Fill {
        if self.age == 0 {
            return self.from;
        }
        const SCALE: usize = 4096;
        let from = self.from.px(SCALE);
        let to = self.to.px(SCALE);
        let left = usize::from(DRAIN_FRAMES - self.age);
        let duration = usize::from(DRAIN_FRAMES);
        // A fast retreat at the start, easing onto the real value.
        let distance = from.saturating_sub(to) * left * left / (duration * duration);
        let edge = to + distance;
        if edge == 0 {
            Fill::EMPTY
        } else {
            Fill::of(edge as u16, SCALE as u16)
        }
    }
}

impl RowEffects {
    fn step(&mut self, before: Option<Row>, after: Option<Row>, drive: bool, v_trigger: bool) {
        let (Some(before), Some(after)) = (before, after) else {
            *self = Self::default();
            return;
        };
        use GaugeKind::*;
        let kind = after.presentation.kind;
        let finished_trigger = v_trigger && before.presentation.kind == Sf5VTrigger && kind == Sf5V;
        let started_trigger = before.presentation.kind == Sf5V && kind == Sf5VTrigger;
        if !before.same_shape(after) || (before.presentation.kind != kind && !finished_trigger) {
            *self = Self::default();
            if started_trigger {
                // Activation has spent the V-Gauge, but its replacement
                // is a timer with a different scale. Accent its label only.
                self.drain = Some(Drain::new(Fill::EMPTY, Fill::EMPTY));
            }
            return;
        }
        if after.presentation.ready.num != 0
            && before.presentation.ready.num != 0
            && after.full()
            && before.full()
        {
            self.flash_phase = (self.flash_phase + 1) % FLASH_CYCLE;
        } else {
            self.flash_phase = 0;
        }
        self.pulse_phase = (self.pulse_phase + 1) % PULSE_CYCLE;
        self.charge = self.charge.and_then(|mut cue| {
            cue.age += 1;
            (cue.age < GaugeCharge::FRAMES
                && after.presentation.ready.num != 0
                && (after.count().is_some() || !more(cue.to, after.presentation.ready)))
            .then_some(cue)
        });
        if let Some(drain) = &mut self.drain {
            drain.age += 1;
            if drain.age == DRAIN_FRAMES {
                self.drain = None;
            }
        }
        if kind != Sf5VTrigger
            && !finished_trigger
            && (!before.can_animate() || !after.can_animate())
        {
            self.charge = None;
            self.drain = None;
            return;
        }
        if let Some(charge) = after.newly_ready(before) {
            self.charge = Some(charge);
            if after.count() > before.count() || !more(before.amount(), after.amount()) {
                // A new stock can arrive while the old payment is still
                // visible. Give the earned cue the row.
                self.drain = None;
            }
        }
        match kind {
            AlphaCustom | Alpha3GuardA | Alpha3GuardV | Alpha3GuardX => return,
            Sf6Drive if !drive => return,
            Sf5VTrigger if !v_trigger => return,
            _ => {}
        }
        let current = after.amount();
        let previous = before.amount();
        let fewer_stocks = after.count() < before.count();
        let fell = after.count() == before.count() && more(previous, current);
        if fewer_stocks || fell {
            self.charge = None;
            // A stock can be spent without changing SFIII's visible
            // remainder. A nonzero subpixel value cues only the numeral.
            if fewer_stocks && !more(previous, current) {
                self.drain = Some(Drain::new(Drain::DIGIT_ONLY, Drain::DIGIT_ONLY));
                return;
            }
            let start = previous;
            let from = self
                .drain
                .map(Drain::edge)
                .filter(|&old| more(old, start))
                .unwrap_or(start);
            self.drain = Some(Drain::new(from, current));
        }
    }

    fn present(self, row: Row) -> RowPresentation {
        let mut info = row.presentation;
        let ready = row.can_animate() && info.ready.num != 0;
        let full = row.full();
        info.flash = ready && full && self.flash_phase < FLASH_ON;
        if ready && !full {
            let angle =
                std::f32::consts::TAU * f32::from(self.pulse_phase) / f32::from(PULSE_CYCLE);
            info.pulse = (54.0 + 150.0 * (1.0 - angle.cos()) * 0.5).round() as u8;
        }
        info.charge = self.charge;
        info.drain = self.drain.map_or(Fill::EMPTY, Drain::edge);
        info.spend_age = self.drain.map(|drain| drain.age);
        info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stock(cells: u8, amount: u16, kind: GaugeKind) -> Row {
        let full = (amount / 48) as u8;
        Row {
            gauge: GaugeRow::stock(cells, full, Fill::of(amount % 48, 48)),
            presentation: RowPresentation::new(kind)
                .with_ready(Fill::of(u16::from(full), u16::from(cells))),
        }
    }

    #[test]
    fn readiness_marks_only_new_stocks_and_expires_while_the_gauge_stays_ready() {
        let stock = |amount| stock(3, amount, GaugeKind::Sf5Critical);
        let mut effects = RowEffects::default();
        effects.step(None, Some(stock(47)), false, false);
        effects.step(Some(stock(47)), Some(stock(48)), false, false);
        let first = effects.present(stock(48));
        assert_eq!(
            first.charge,
            Some(GaugeCharge {
                from: Fill::of(0, 3),
                to: Fill::of(1, 3),
                age: 0
            })
        );
        for age in 1..=GaugeCharge::FRAMES {
            effects.step(Some(stock(48)), Some(stock(48)), false, false);
            assert_eq!(
                effects.present(stock(48)).charge.map(|cue| cue.age),
                (age < GaugeCharge::FRAMES).then_some(age)
            );
        }
        assert_ne!(effects.present(stock(48)).pulse, 0);

        effects.step(Some(stock(95)), Some(stock(96)), false, false);
        assert_eq!(
            effects.present(stock(96)).charge,
            Some(GaugeCharge {
                from: Fill::of(1, 3),
                to: Fill::of(2, 3),
                age: 0,
            })
        );
        effects.step(Some(stock(96)), Some(stock(48)), false, false);
        let spent = effects.present(stock(48));
        assert_eq!(spent.charge, None);
        assert_eq!(spent.spend_age, Some(0));
        assert_eq!(spent.drain.px(96), 64);
        for age in 1..12 {
            effects.step(Some(stock(48)), Some(stock(48)), false, false);
            assert_eq!(effects.present(stock(48)).spend_age, Some(age));
        }
        effects.step(Some(stock(48)), Some(stock(48)), false, false);
        let quiet = effects.present(stock(48));
        assert_eq!(quiet.spend_age, None);
        assert_eq!(quiet.drain, Fill::EMPTY);

        effects.step(Some(stock(96)), Some(stock(47)), false, false);
        effects.step(Some(stock(47)), Some(stock(48)), false, false);
        let recharged = effects.present(stock(48));
        assert_eq!(recharged.charge.unwrap().age, 0);
        assert_eq!(recharged.spend_age, None);
    }

    #[test]
    fn earned_drive_readiness_and_paid_drain_ignore_ordinary_regeneration_and_loss() {
        let drive = |amount| stock(6, amount, GaugeKind::Sf6Drive);
        let mut effects = RowEffects::default();
        effects.step(Some(drive(47)), Some(drive(48)), false, false);
        assert_eq!(effects.present(drive(48)).charge.unwrap().age, 0);
        effects.step(Some(drive(48)), Some(drive(49)), false, false);
        assert_eq!(effects.present(drive(49)).charge.unwrap().age, 1);
        effects.step(Some(drive(49)), Some(drive(48)), false, false);
        assert!(effects.present(drive(48)).spend_age.is_none());
        effects.step(Some(drive(48)), Some(drive(10)), true, false);
        assert_eq!(effects.present(drive(10)).spend_age, Some(0));
        assert!(effects.present(drive(10)).charge.is_none());

        let bar = |points, ready| Row {
            gauge: GaugeRow::bar(Fill::of(points, 100), Fill::FULL),
            presentation: RowPresentation::new(GaugeKind::Sf4Revenge).with_ready(ready),
        };
        let half = Fill::of(50, 100);
        let growing = Fill::of(51, 100);
        effects.step(
            Some(bar(49, Fill::EMPTY)),
            Some(bar(50, half)),
            false,
            false,
        );
        effects.step(Some(bar(50, half)), Some(bar(51, growing)), false, false);
        assert_eq!(effects.present(bar(51, growing)).charge.unwrap().age, 1);
    }

    #[test]
    fn third_strike_level_up_survives_a_wrapped_remainder_and_loses_to_a_spend() {
        let row = |level, points| Row {
            gauge: GaugeRow::level(level, 3, Fill::of(points, 48)),
            presentation: RowPresentation::new(GaugeKind::Sf3ThirdStrike)
                .with_ready(if level == 0 { Fill::EMPTY } else { Fill::FULL }),
        };
        let mut effects = RowEffects::default();
        effects.step(Some(row(1, 47)), Some(row(2, 0)), false, false);
        assert_eq!(
            effects.present(row(2, 0)).charge,
            Some(GaugeCharge {
                from: Fill::EMPTY,
                to: Fill::FULL,
                age: 0,
            })
        );
        effects.step(Some(row(2, 0)), Some(row(1, 2)), false, false);
        let paid = effects.present(row(1, 2));
        assert_eq!(paid.charge, None);
        assert_eq!(paid.spend_age, Some(0));
        assert_eq!(paid.drain.px(96), 0);
    }

    #[test]
    fn v_trigger_tail_comes_from_paid_energy_and_survives_a_paid_expiration() {
        let timed = |amount| Row {
            gauge: GaugeRow::stock(2, 0, Fill::of(amount, 48)).with_state(GaugeState::Draining),
            presentation: RowPresentation::new(GaugeKind::Sf5VTrigger),
        };
        let mut effects = RowEffects::default();
        let ready = stock(2, 96, GaugeKind::Sf5V);
        effects.step(Some(ready), Some(timed(45)), false, false);
        assert_eq!(effects.present(timed(45)).spend_age, Some(0));
        effects.step(Some(timed(45)), Some(timed(44)), false, false);
        assert_eq!(effects.present(timed(44)).drain, Fill::EMPTY);

        effects.step(Some(timed(44)), Some(timed(20)), false, true);
        let start = effects.present(timed(20)).drain;
        assert!(more(start, timed(20).amount()));

        effects.step(Some(timed(20)), Some(timed(19)), false, false);
        let next = effects.present(timed(19)).drain;
        assert!(more(start, next) && more(next, timed(19).amount()));

        let empty = Row {
            gauge: GaugeRow::stock(2, 0, Fill::EMPTY),
            presentation: RowPresentation::new(GaugeKind::Sf5V),
        };
        effects.step(Some(timed(19)), Some(empty), false, true);
        assert!(more(effects.present(empty).drain, Fill::EMPTY));
        for _ in 0..DRAIN_FRAMES {
            effects.step(Some(empty), Some(empty), false, false);
        }
        assert_eq!(effects.present(empty).drain, Fill::EMPTY);
    }
}
