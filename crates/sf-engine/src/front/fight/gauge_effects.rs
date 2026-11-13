//! Presentation history for the real Fight HUD. Meter amounts come from the
//! machine before and after a step, after any external Training refill.

use sf_video::era_hud::{
    Fill, GaugeKind, GaugePresentation, GaugeRow, GaugeState, RowPresentation,
};

const DRAIN_FRAMES: u8 = 27;
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
    drain: Option<Drain>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Drain {
    from: Fill,
    to: Fill,
    age: u8,
}

impl Drain {
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
        Fill::of((to + distance) as u16, SCALE as u16)
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
        if !before.same_shape(after) || (before.presentation.kind != kind && !finished_trigger) {
            *self = Self::default();
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
            self.drain = None;
            return;
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
            // A stock can be spent without changing SFIII's visible
            // remainder. A nonzero subpixel value cues only the numeral.
            if fewer_stocks && !more(previous, current) {
                let marker = Fill::of(1, 4096);
                self.drain = Some(Drain {
                    from: marker,
                    to: marker,
                    age: 0,
                });
                return;
            }
            let start = previous;
            let from = self
                .drain
                .map(Drain::edge)
                .filter(|&old| more(old, start))
                .unwrap_or(start);
            self.drain = Some(Drain {
                from,
                to: current,
                age: 0,
            });
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
        info.drain = self.drain.map_or(Fill::EMPTY, Drain::edge);
        info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v_trigger_tail_comes_from_paid_energy_and_survives_a_paid_expiration() {
        let timed = |amount| Row {
            gauge: GaugeRow::stock(2, 0, Fill::of(amount, 48)).with_state(GaugeState::Draining),
            presentation: RowPresentation::new(GaugeKind::Sf5VTrigger),
        };
        let mut effects = RowEffects::default();
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
