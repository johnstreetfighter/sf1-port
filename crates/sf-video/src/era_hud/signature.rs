//! Era colors and their emblem labels, independent of the gauge geometry.

use super::{lit_pen, Corner, Fill, GaugeRow, GaugeState, Ink, Side, Slot};
use crate::render::{HEIGHT, WIDTH};

/// The native gauge a row describes. This stays separate from the row's
/// geometry, since several different mechanics draw the same stock bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GaugeKind {
    Unified,
    Sf2Super,
    Alpha1Super,
    Alpha2Super,
    AlphaCustom,
    Alpha3A,
    Alpha3V,
    Alpha3X,
    Alpha3GuardA,
    Alpha3GuardV,
    Alpha3GuardX,
    Sf3NewGeneration,
    Sf3SecondImpact,
    Sf3ThirdStrike,
    Sf4Super,
    Sf4Revenge,
    Sf5Critical,
    Sf5V,
    Sf5VTrigger,
    Sf6Drive,
    Sf6SuperArt,
}

/// The presentation of one corner's main row and optional lower row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GaugePresentation {
    pub main: RowPresentation,
    pub second: Option<RowPresentation>,
}

/// A presentation sample from the fight. Amounts are fractions of the
/// rendered row. A drain is a trailing cutoff behind the actual fill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowPresentation {
    pub kind: GaugeKind,
    pub ready: Fill,
    pub flash: bool,
    pub pulse: u8,
    pub drain: Fill,
    /// Most recent earned readiness, while its snap or sweep is visible.
    pub charge: Option<GaugeCharge>,
    /// Elapsed frames since the last discrete payment, including a payment
    /// with no drawable discharge trail.
    pub spend_age: Option<u8>,
}

/// The newly usable span of a row, measured from its outer end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GaugeCharge {
    pub from: Fill,
    pub to: Fill,
    pub age: u8,
}

impl GaugeCharge {
    pub const FRAMES: u8 = 13;
}

impl RowPresentation {
    pub const fn new(kind: GaugeKind) -> Self {
        Self {
            kind,
            ready: Fill::EMPTY,
            flash: false,
            pulse: 0,
            drain: Fill::EMPTY,
            charge: None,
            spend_age: None,
        }
    }

    pub const fn with_ready(mut self, ready: Fill) -> Self {
        self.ready = ready;
        self
    }
}

/// The fill cell and the position within it. Each cell counts as one stock
/// even when the art's geometry makes it a pixel wider than its neighbors.
#[derive(Clone, Copy)]
pub(super) struct GaugeColumn {
    pub lit: bool,
    index: usize,
    width: usize,
    cell: usize,
    cells: usize,
}

impl GaugeColumn {
    pub(super) fn new(index: usize, width: usize, lit: usize, cell: usize, cells: usize) -> Self {
        Self { lit: index < lit, index, width, cell, cells }
    }

    pub(super) fn continuous(index: usize, width: usize, lit: usize) -> Self {
        Self::new(index, width, lit, 0, 1)
    }

    fn inside(self, fill: Fill) -> bool {
        self.cell * self.width + self.index < fill.px(self.width * self.cells)
    }

    fn near_edge(self, fill: Fill) -> bool {
        let at = self.cell * self.width + self.index;
        at + 2 >= fill.px(self.width * self.cells)
    }

    fn charge_span(self, charge: GaugeCharge) -> std::ops::Range<usize> {
        let unit = |fill: Fill| {
            fill.px(self.width * self.cells)
                .saturating_sub(self.cell * self.width)
                .min(self.width)
        };
        unit(charge.from)..unit(charge.to)
    }

    fn spend_spark(self, fill: Fill) -> bool {
        let at = self.cell * self.width + self.index;
        at % 3 == 0 && at + 7 >= fill.px(self.width * self.cells)
    }
}

const fn color(rgb: u32) -> u32 {
    0xff_00_00_00 | rgb
}

/// Representative fill, highlight, rail and unfilled track colors, sampled
/// from the arcade editions and Capcom's SFV/SF6 manuals. The art still
/// uses its existing caps, dividers, thickness and rounding.
#[derive(Clone, Copy)]
struct Palette {
    body: u32,
    light: u32,
    rail: u32,
    track: u32,
}

const fn palette(body: u32, light: u32, rail: u32, track: u32) -> Palette {
    Palette { body: color(body), light: color(light), rail: color(rail), track: color(track) }
}

impl GaugeKind {
    fn palette(self, side: Side) -> Palette {
        match self {
            Self::Unified => palette(0xcc9900, 0xffcc77, 0x9999ff, 0x444444),
            Self::Sf2Super => palette(0xffaa01, 0xffd077, 0xffffff, 0x7798db),
            Self::Alpha1Super => palette(0x88ff77, 0xbbffbb, 0xff5522, 0x770088),
            Self::Alpha2Super | Self::AlphaCustom => palette(0x33dd55, 0x88ff99, 0x334466, 0x117733),
            Self::Alpha3A => palette(0x00ff00, 0x99ff88, 0x000044, 0x046201),
            Self::Alpha3V => palette(0x2478b5, 0x99eeff, 0x000044, 0x117799),
            Self::Alpha3X => palette(0xe90c03, 0xffaa00, 0x000044, 0x6e1a02),
            Self::Alpha3GuardA => palette(0x00aa66, 0x72ffd4, 0x000044, 0x103b2c),
            Self::Alpha3GuardV => palette(0x7777ff, 0xbbbbff, 0x000044, 0x242450),
            Self::Alpha3GuardX => palette(0xff3344, 0xff9ea7, 0x000044, 0x481526),
            Self::Sf3NewGeneration => palette(0xffff00, 0xffff89, 0x5a94ff, 0x29085a),
            Self::Sf3SecondImpact if side == Side::P1 => palette(0x159eef, 0x74caff, 0xc8c0d0, 0x403050),
            Self::Sf3SecondImpact => palette(0x16cbb6, 0x82ecdd, 0xc8c0d0, 0x403050),
            Self::Sf3ThirdStrike => palette(0x0ec9f2, 0x12fbf8, 0x97a3b9, 0x291a2d),
            Self::Sf4Super => palette(0x0a8bff, 0x21d8ff, 0xd5d5d5, 0x332f30),
            Self::Sf4Revenge => palette(0x44da21, 0x80ff67, 0x778c76, 0x283629),
            Self::Sf5Critical => palette(0x46afc3, 0xabeafe, 0xa9d4de, 0x183441),
            Self::Sf5V | Self::Sf5VTrigger => palette(0xe33033, 0xf59aa6, 0xe6afb7, 0x471d26),
            Self::Sf6Drive => palette(0x2ac400, 0xe7e307, 0xb3d391, 0x223421),
            Self::Sf6SuperArt if side == Side::P1 => palette(0xfa79b3, 0xffd4eb, 0xe5b2cc, 0x4b2037),
            Self::Sf6SuperArt => palette(0x217ed3, 0xc6faff, 0xa5d4f0, 0x17334b),
        }
    }

    fn label(self, state: GaugeState) -> &'static str {
        match self {
            Self::Unified | Self::Sf2Super | Self::Alpha1Super | Self::Alpha2Super | Self::Sf4Super => "SUPER",
            Self::AlphaCustom => "CUSTOM",
            Self::Alpha3A => "A-ISM",
            Self::Alpha3V => "V-ISM",
            Self::Alpha3X => "X-ISM",
            Self::Alpha3GuardA | Self::Alpha3GuardV | Self::Alpha3GuardX => "GUARD",
            Self::Sf3NewGeneration | Self::Sf3SecondImpact | Self::Sf3ThirdStrike | Self::Sf6SuperArt => "SUPER ART",
            Self::Sf4Revenge => "REVENGE",
            Self::Sf5Critical => "EX / CA",
            Self::Sf5V => "V-GAUGE",
            Self::Sf5VTrigger => "V-TRIGGER",
            Self::Sf6Drive if state == GaugeState::Greyed => "BURNOUT",
            Self::Sf6Drive => "DRIVE",
        }
    }

    fn emblem(self) -> [u8; 5] {
        match self {
            Self::Unified => [4, 6, 7, 6, 4],
            Self::Sf2Super => [7, 0, 7, 0, 7],
            Self::Alpha1Super | Self::Alpha2Super | Self::AlphaCustom
            | Self::Alpha3A | Self::Alpha3V | Self::Alpha3X
            | Self::Alpha3GuardA | Self::Alpha3GuardV | Self::Alpha3GuardX => [1, 2, 4, 2, 1],
            Self::Sf3NewGeneration | Self::Sf3SecondImpact | Self::Sf3ThirdStrike => [2, 7, 7, 7, 2],
            Self::Sf4Super | Self::Sf4Revenge => [1, 3, 7, 6, 4],
            Self::Sf5Critical | Self::Sf5V | Self::Sf5VTrigger => [1, 3, 2, 6, 4],
            Self::Sf6Drive | Self::Sf6SuperArt => [2, 5, 5, 5, 2],
        }
    }
}

const WHITE: u32 = color(0xffffff);
const DRAIN_AMBER: u32 = color(0xffb960);
const BURNOUT: Palette = palette(0x929290, 0xdbdbd9, 0xa3a4a4, 0x4f5151);

fn mix(a: u32, b: u32, amount: u8) -> u32 {
    let weight = u32::from(amount);
    [0, 8, 16].into_iter().fold(color(0), |argb, shift| {
        let from = (a >> shift) & 0xff;
        let to = (b >> shift) & 0xff;
        argb | ((from * (255 - weight) + to * weight + 127) / 255) << shift
    })
}

#[derive(Clone, Copy)]
pub(super) struct RowPaint {
    row: GaugeRow,
    presentation: Option<RowPresentation>,
    palette: Palette,
    state: GaugeState,
}

impl RowPaint {
    pub(super) fn new(row: GaugeRow, presentation: Option<RowPresentation>, side: Side) -> Self {
        let state = row.state();
        let palette = presentation.map_or(BURNOUT, |p| p.kind.palette(side));
        Self { row, presentation, palette, state }
    }

    fn shown(self) -> Palette {
        if self.state == GaugeState::Greyed { BURNOUT } else { self.palette }
    }

    pub(super) fn rail(self, original: u32) -> u32 {
        if self.presentation.is_some() { self.shown().rail } else { original }
    }

    pub(super) fn light_rail(self, original: u32) -> u32 {
        if self.presentation.is_some() { mix(self.shown().rail, self.shown().light, 102) } else { original }
    }

    pub(super) fn dark_rail(self, original: u32) -> u32 {
        if self.presentation.is_some() { mix(self.shown().rail, self.shown().track, 160) } else { original }
    }

    pub(super) fn shadow(self, original: u32) -> u32 {
        if self.presentation.is_some() { mix(self.shown().track, color(0x14141d), 96) } else { original }
    }

    pub(super) fn track(self, original: u32) -> u32 {
        if self.presentation.is_some() { self.shown().track } else { original }
    }

    fn full(self) -> bool {
        match self.row {
            GaugeRow::Stock { full, cells, .. } => full >= cells,
            GaugeRow::Level { state, .. } => state == GaugeState::Full,
            GaugeRow::Bar { fill, .. } => fill.px(1) != 0,
        }
    }

    pub(super) fn charge_rail(self, column: GaugeColumn, original: u32) -> u32 {
        // A level-up can wrap the progress to zero. Sweep its rail too so
        // the new stock reads even when there is no remainder to light.
        if matches!(self.row, GaugeRow::Level { .. }) {
            if let Some(ink) = self.sweep_ink(column, self.shown().light) {
                return ink;
            }
        }
        self.snap_ink().filter(|(_, charge)| column.charge_span(*charge).contains(&column.index))
            .map_or(original, |(ink, _)| ink)
    }

    pub(super) fn charge_bevel(self, column: GaugeColumn, original: (u32, u32)) -> (u32, u32) {
        if let Some(ink) = self.charge_ink(column) {
            (ink, mix(original.1, ink, 135))
        } else {
            original
        }
    }

    pub(super) fn charge_ink(self, column: GaugeColumn) -> Option<u32> {
        self.snap_ink().filter(|(_, charge)| !column.charge_span(*charge).is_empty())
            .map(|(ink, _)| ink)
    }

    fn snap_ink(self) -> Option<(u32, GaugeCharge)> {
        let charge = self.presentation?.charge?;
        (charge.age < 8)
            .then(|| if charge.age < 4 { super::pens::CREAM } else { self.shown().light })
            .map(|ink| (ink, charge))
    }

    fn sweep_ink(self, column: GaugeColumn, light: u32) -> Option<u32> {
        let charge = self.presentation?.charge.filter(|cue| cue.age < GaugeCharge::FRAMES)?;
        let span = column.charge_span(charge);
        let head = span.start + usize::from(charge.age) * span.len() / usize::from(GaugeCharge::FRAMES);
        (span.contains(&column.index) && column.index <= head && column.index + 2 >= head)
            .then_some(if column.index == head { super::pens::CREAM } else { light })
    }

    fn filling_palette(self, p: RowPresentation, column: GaugeColumn) -> Palette {
        let Palette { mut body, mut light, rail, track } = self.shown();
        if self.state != GaugeState::Greyed {
            match p.kind {
                GaugeKind::Alpha1Super if p.ready.num == 0 => {
                    body = color(0x00bbff);
                    light = color(0xb5edff);
                }
                GaugeKind::Alpha2Super if self.full() => {
                    body = color(0xffb331);
                    light = color(0xffff33);
                }
                GaugeKind::Alpha2Super if column.cell == 0 => {
                    body = color(0x44ccff);
                    light = color(0xade9ff);
                }
                GaugeKind::Alpha3V if self.full() => body = color(0x99eeff),
                GaugeKind::Alpha3X if self.full() => light = color(0xffdd00),
                GaugeKind::Sf4Revenge if p.ready.num != 0 => {
                    body = color(0xfeff03);
                    light = color(0xffff93);
                }
                GaugeKind::Sf6Drive => {
                    const STOCKS: [u32; 6] = [0x2ac400, 0x2ac400, 0x4fce00, 0x77cf00, 0xb6d514, 0xd0d204];
                    body = color(STOCKS[column.cell.min(STOCKS.len() - 1)]);
                    light = mix(body, color(0xf3f795), 126);
                }
                _ => {}
            }
        }
        Palette { body, light, rail, track }
    }

    pub(super) fn digit(self, original: [u32; 3]) -> [u32; 3] {
        let Some(p) = self.presentation else { return original };
        let Palette { body, light, .. } = self.shown();
        let dark = self.shadow(original[1]);
        let stocked = matches!(self.row, GaugeRow::Level { level: 1.., .. });
        if p.drain.num != 0 {
            return [DRAIN_AMBER, dark, mix(body, DRAIN_AMBER, 154)];
        }
        if let Some((ink, _)) = self.snap_ink() {
            return [ink, dark, mix(body, ink, 154)];
        }
        if p.flash && self.full() {
            let flash = mix(light, WHITE, 102);
            return [flash, dark, flash];
        }
        let pulse = if stocked && p.ready.num != 0 && !self.full() { p.pulse } else { 0 };
        [mix(light, WHITE, pulse), dark, mix(body, light, pulse)]
    }

    pub(super) fn fill(self, ink: Ink, column: GaugeColumn, fy: usize, empty: u32) -> u32 {
        let Some(p) = self.presentation else {
            return if column.lit { lit_pen(self.state, ink, fy) } else { empty };
        };
        let Palette { body, light, track, .. } = self.filling_palette(p, column);
        if !column.lit {
            if column.inside(p.drain) {
                if fy == 0 && p.spend_age.is_some_and(|age| age < 7) && column.spend_spark(p.drain) {
                    return super::pens::CREAM;
                }
                let tail = mix(track, DRAIN_AMBER, 122);
                return if column.near_edge(p.drain) { DRAIN_AMBER } else { tail };
            }
            return track;
        }
        if self.state == GaugeState::Greyed {
            return if fy == 0 { light } else { body };
        }
        if self.state == GaugeState::Draining {
            return mix(body, track, 90);
        }
        if let Some(ink) = self.sweep_ink(column, light) {
            return ink;
        }
        let counter_lit = p.kind != GaugeKind::Sf4Revenge && self.state == GaugeState::Flashing { lit: true };
        if (self.full() && p.flash) || counter_lit {
            return match p.kind {
                GaugeKind::Alpha3X => color(0xffaa00),
                GaugeKind::Sf5Critical => color(0xf8fcff),
                _ => mix(light, WHITE, 102),
            };
        }
        let digit_owns_ready = matches!(self.row, GaugeRow::Level { level: 1.., .. });
        if !self.full() && !digit_owns_ready && column.inside(p.ready) {
            return if fy == 0 {
                mix(body, light, 96 + p.pulse / 2)
            } else {
                mix(body, light, p.pulse / 2)
            };
        }
        if self.full() && fy == 0 { light } else { body }
    }
}

fn glyph(letter: u8) -> [u8; 5] {
    match letter {
        b'A' => [2, 5, 7, 5, 5], b'B' => [6, 5, 6, 5, 6], b'C' => [3, 4, 4, 4, 3],
        b'D' => [6, 5, 5, 5, 6], b'E' => [7, 4, 6, 4, 7], b'G' => [3, 4, 5, 5, 3],
        b'I' => [7, 2, 2, 2, 7], b'M' => [5, 7, 7, 5, 5], b'N' => [5, 7, 7, 7, 5],
        b'O' => [2, 5, 5, 5, 2], b'P' => [6, 5, 6, 4, 4], b'R' => [6, 5, 6, 5, 5],
        b'S' => [3, 4, 2, 1, 6], b'T' => [7, 2, 2, 2, 2], b'U' => [5, 5, 5, 5, 7],
        b'V' => [5, 5, 5, 5, 2], b'X' => [5, 5, 2, 5, 5], b'-' => [0, 0, 7, 0, 0],
        b'/' => [1, 1, 2, 4, 4], _ => [0; 5],
    }
}

pub(super) fn draw_label(corner: &mut Corner<'_>, lx: usize, at: Slot, kind: GaugeKind, paint: RowPaint) {
    const BACKING: u32 = color(0x171720);
    const LETTERING: u32 = color(0xfbf7ed);
    let label = kind.label(paint.state);
    let width = label.len() * 4 + 9;
    let height = at.height.max(7);
    let y = at.y0.saturating_sub(height - at.height);
    let x = corner.text_x(lx, width);
    // Compact rows borrow a line from above, leaving the Training frame meter
    // and the independent ally's neighboring corner clear below.
    if x + width > WIDTH || y + height > HEIGHT {
        return;
    }
    for py in y..y + height {
        corner.argb[py * WIDTH + x..py * WIDTH + x + width].fill(BACKING);
    }

    let mirrored = corner.side == Side::P2;
    let mark_x = if mirrored { x + width - 5 } else { x + 2 };
    let text_x = if mirrored { x + 2 } else { x + 8 };
    let normal_accent = match kind {
        GaugeKind::Alpha3A => color(0x7fef76),
        GaugeKind::Alpha3X => color(0xff8c7e),
        GaugeKind::Sf6Drive if paint.state != GaugeState::Greyed => color(0x9af16e),
        _ => paint.shown().light,
    };
    let cue = paint.presentation.and_then(|info| {
        if info.spend_age.is_some_and(|age| age < 10) {
            Some(DRAIN_AMBER)
        } else if info.charge.is_some_and(|charge| charge.age < 10) {
            Some(super::pens::CREAM)
        } else {
            None
        }
    });
    let mut draw = |gx, bitmap: [u8; 5], ink, flip| {
        for (row, bits) in bitmap.into_iter().enumerate() {
            for col in 0..3 {
                let bit = if flip { 1 << col } else { 4 >> col };
                if bits & bit != 0 {
                    corner.argb[(y + 1 + row) * WIDTH + gx + col] = ink;
                }
            }
        }
    };
    draw(mark_x, kind.emblem(), cue.unwrap_or_else(|| mix(normal_accent, color(0xfff7e1), 48)), mirrored);
    for (index, letter) in label.bytes().enumerate() {
        draw(text_x + index * 4, glyph(letter), LETTERING, false);
    }
    if let Some(ink) = cue {
        let bottom = (y + height - 1) * WIDTH + x;
        corner.argb[bottom..bottom + width].fill(ink);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::era_hud::{
        draw_lifted, draw_lifted_presented, EraGauge, EraHud, GaugeArt, CORNER_EDGE,
        MAIN_ROW, SIGNATURE_CORNER_WIDTH,
    };

    const BACKDROP: u32 = color(0x090809);

    #[test]
    fn outlined_signatures_keep_the_old_gauge_footprint_and_mirror_only_the_emblem() {
        let gauge = EraGauge::two(
            GaugeRow::stock(1, 0, Fill::of(2, 3)),
            GaugeRow::bar(Fill::of(1, 2), Fill::FULL),
        );
        let hud = EraHud { p1: Some(gauge), p2: Some(gauge) };
        let signature = Some(GaugePresentation {
            main: RowPresentation::new(GaugeKind::Alpha3X),
            second: Some(RowPresentation::new(GaugeKind::Alpha3GuardX)),
        });
        let lift = 12;
        let bottom = HEIGHT - 8 - lift;
        for art in GaugeArt::ALL {
            let mut plain = vec![BACKDROP; WIDTH * HEIGHT];
            let mut decorated = plain.clone();
            draw_lifted(&mut plain, &hud, art, None, lift);
            draw_lifted_presented(&mut decorated, &hud, art, None, lift, [signature; 2]);

            for (bar_y, label_y, height) in [(bottom - 16, bottom - 16, 8), (bottom - 6, bottom - 7, 7)] {
                let end = (8..WIDTH / 2).find(|&x| plain[bar_y * WIDTH + x] == BACKDROP).unwrap();
                let label_width = 29; // Five glyphs, their emblem, and the border.
                let left = end + 1;
                let right = WIDTH - left - label_width;
                let at = |x, y| decorated[y * WIDTH + x];
                let label_backing = color(0x171720);
                for label_x in [left, right] {
                    assert!((0..label_width).all(|x|
                        at(label_x + x, label_y) == label_backing &&
                        at(label_x + x, label_y + height - 1) == label_backing
                    ));
                    assert!((0..height).all(|y|
                        at(label_x, label_y + y) == label_backing &&
                        at(label_x + label_width - 1, label_y + y) == label_backing
                    ));
                }
                for y in label_y + 1..label_y + 6 {
                    for x in 0..3 {
                        assert_eq!(at(left + 2 + x, y), at(right + label_width - 3 - x, y));
                    }
                    for x in 0..19 {
                        assert_eq!(at(left + 8 + x, y), at(right + 2 + x, y));
                    }
                }
                assert_ne!(at(left + 2, label_y + 1), at(left + 4, label_y + 1));
            }
            assert!(decorated[bottom * WIDTH..].iter().all(|&p| p == BACKDROP));
        }
    }

    #[test]
    fn only_usable_stock_pulses_and_a_spend_retreats_toward_the_live_fill() {
        let gauge = EraGauge::one(GaugeRow::stock(3, 1, Fill::of(1, 4)));
        let hud = EraHud { p1: Some(gauge), p2: None };
        let ready = RowPresentation::new(GaugeKind::Sf5Critical).with_ready(Fill::of(1, 3));
        let draw = |sample| {
            let mut frame = vec![BACKDROP; WIDTH * HEIGHT];
            let presentation = GaugePresentation { main: sample, second: None };
            draw_lifted_presented(&mut frame, &hud, GaugeArt::Ruled, None, 0, [Some(presentation), None]);
            frame
        };
        let idle = draw(ready);
        let moving = draw(RowPresentation { pulse: 220, drain: Fill::of(2, 3), ..ready });
        let returning = draw(RowPresentation { pulse: 220, drain: Fill::of(1, 2), ..ready });
        let sample = |x| (HEIGHT - 14) * WIDTH + x;
        assert_ne!(moving[sample(24)], idle[sample(24)]); // Ready stock.
        assert_eq!(moving[sample(43)], idle[sample(43)]); // Partial stock.
        assert_ne!(moving[sample(64)], idle[sample(64)]); // Spent part.
        assert_eq!(returning[sample(64)], idle[sample(64)]);
        assert_eq!(moving[sample(72)], idle[sample(72)]); // Divider.
        assert_eq!(moving[sample(73)], idle[sample(73)]); // Outside spend.
    }

    #[test]
    fn charge_and_spend_stay_inside_the_existing_mirrored_row_and_label_for_each_art() {
        let row = GaugeRow::stock(3, 2, Fill::EMPTY);
        let gauge = EraGauge::one(row);
        let hud = EraHud { p1: Some(gauge), p2: Some(gauge) };
        let ready = RowPresentation::new(GaugeKind::Sf5Critical).with_ready(Fill::of(2, 3));
        let charge = GaugeCharge { from: Fill::of(1, 3), to: Fill::of(2, 3), age: 0 };
        let top = HEIGHT - 8 - MAIN_ROW;
        for art in GaugeArt::ALL {
            let draw = |sample| {
                let mut frame = vec![BACKDROP; WIDTH * HEIGHT];
                let presentation = GaugePresentation { main: sample, second: None };
                draw_lifted_presented(&mut frame, &hud, art, None, 0, [Some(presentation); 2]);
                frame
            };
            let steady = draw(ready);
            let snap = draw(RowPresentation { charge: Some(charge), ..ready });
            let sweep = draw(RowPresentation { charge: Some(GaugeCharge { age: 11, ..charge }), ..ready });
            let expense = draw(RowPresentation { drain: Fill::of(5, 6), spend_age: Some(0), ..ready });
            let label_start = (8..WIDTH / 2).find(|&x| steady[(top + MAIN_ROW - 1) * WIDTH + x] == color(0x171720)).unwrap();
            for frame in [&snap, &sweep, &expense] {
                assert!(frame[..top * WIDTH].iter().all(|&pixel| pixel == BACKDROP));
                assert!(frame[(top + MAIN_ROW) * WIDTH..].iter().all(|&pixel| pixel == BACKDROP));
                for y in top..top + MAIN_ROW {
                    assert_eq!(frame[y * WIDTH + 13], steady[y * WIDTH + 13], "older stock of {art:?}");
                    assert_eq!(frame[y * WIDTH + WIDTH - 14], steady[y * WIDTH + WIDTH - 14]);
                }
                for (&sample, &plain) in frame.iter().zip(&steady) {
                    if plain == BACKDROP {
                        assert_eq!(sample, plain, "{art:?} effect stays within the current footprint");
                    }
                }
            }
            let at = (top + MAIN_ROW - 1) * WIDTH + label_start;
            assert_eq!(snap[at], super::super::pens::CREAM);
            assert_eq!(sweep[at], steady[at]);
            assert_eq!(expense[at], DRAIN_AMBER);
            let body = top * WIDTH..(top + MAIN_ROW - 1) * WIDTH;
            assert_ne!(&snap[body.clone()], &steady[body.clone()]);
            assert_ne!(&sweep[body.clone()], &steady[body.clone()]);
            assert_ne!(&expense[body.clone()], &steady[body]);
        }
    }

    #[test]
    fn a_third_strike_stock_spend_with_a_steady_remainder_marks_only_the_digit() {
        let row = GaugeRow::level(1, 3, Fill::of(1, 2));
        let kind = GaugeKind::Sf3ThirdStrike;
        let quiet = RowPaint::new(row, Some(RowPresentation::new(kind)), Side::P1);
        let marker = RowPresentation { drain: Fill::of(1, 4096), ..RowPresentation::new(kind) };
        let spending = RowPaint::new(row, Some(marker), Side::P1);
        assert_ne!(quiet.digit(super::super::pens::DIGIT_GOLD), spending.digit(super::super::pens::DIGIT_GOLD));
        for i in 0..96 {
            let column = GaugeColumn::continuous(i, 96, 48);
            assert_eq!(
                quiet.fill(Ink::GOLD, column, 0, BACKDROP),
                spending.fill(Ink::GOLD, column, 0, BACKDROP),
            );
        }

        let gauge = EraGauge::one(row);
        let mut frame = vec![BACKDROP; WIDTH * HEIGHT];
        draw_lifted_presented(
            &mut frame,
            &EraHud { p1: Some(gauge), p2: None },
            GaugeArt::Plate,
            None,
            0,
            [Some(GaugePresentation { main: marker, second: None }), None],
        );
        let last = CORNER_EDGE + SIGNATURE_CORNER_WIDTH - 1;
        let row = &frame[(HEIGHT - 16) * WIDTH..(HEIGHT - 15) * WIDTH];
        assert_eq!(row[last], color(0x171720));
        assert!(row[last + 1..].iter().all(|&pixel| pixel == BACKDROP));
    }
}
