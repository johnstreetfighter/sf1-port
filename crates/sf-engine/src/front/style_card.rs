//! The three spinners inside a Character Select portrait.

use sf_video::compositor::{darken, fallback_text, ink, outline, RomFont};
use sf_video::render::WIDTH;

use crate::dramatic::DramaticRules;
use crate::era::{
    listed_families, listed_revisions, loadouts, EraFamily, Loadout, Revision, StylePick,
};
use crate::roster::{MenuInput, TournamentFighter};
use crate::era_fight::sf3::style_art;

use super::draw::fit;

const ERA_ORDER: [EraFamily; 8] = [
    EraFamily::Sf1Plus,
    EraFamily::Sf2,
    EraFamily::SfZ,
    EraFamily::Sf3,
    EraFamily::Sf4,
    EraFamily::Sf5,
    EraFamily::Sf6,
    EraFamily::Sf1987,
];
const ADVANCE: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StyleLatch {
    Open(u32),
    Lock(u32),
}

impl StyleLatch {
    pub(super) fn paint(self, argb: &mut [u32], (x, y, w, h): (usize, usize, usize, usize), side: usize, frame: u32) {
        let rail = [0xffca_d8ff, 0xffff_d2bc][side];
        match self {
            Self::Open(start) => {
                let age = frame.wrapping_sub(start) as usize;
                if age >= 7 {
                    return;
                }
                let row = &mut argb[(y + 7 + 14 * age) * WIDTH + x..][..w];
                row[3..5].fill(rail);
                row[w - 4..w - 2].fill(rail);
                for pixel in &mut row[5..w - 4] {
                    // A pale 18% pass leaves the current values legible.
                    let source = *pixel;
                    *pixel = [16, 8, 0].into_iter().zip([220, 230, 255]).fold(0xff00_0000, |color, (shift, ink)| {
                        let value = ((source >> shift & 0xff) * 82 + ink * 18) / 100;
                        color | (value << shift)
                    });
                }
            }
            Self::Lock(start) => {
                let age = frame.wrapping_sub(start) as usize;
                if age >= 8 {
                    return;
                }
                let top = 2 + (14 * age).min(h - 10);
                for dy in top..top + 5 {
                    let row = &mut argb[(y + dy) * WIDTH + x..][..w];
                    row[..2].fill(rail);
                    row[w - 2..].fill(rail);
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct StyleCard {
    fighter: TournamentFighter,
    what_if: bool,
    dramatic: bool,
    families: Vec<EraFamily>,
    revisions: Vec<Revision>,
    loadouts: Vec<Loadout>,
    pick: StylePick,
    row: usize,
}

impl StyleCard {
    pub(super) fn from_pick(
        fighter: TournamentFighter,
        pick: StylePick,
        what_if: bool,
    ) -> Option<Self> {
        if !pick.available_for(fighter, what_if) {
            return None;
        }
        let mut card = Self::open(fighter, pick.family, what_if);
        card.set_revision(pick.revision);
        card.pick = pick;
        Some(card)
    }

    pub(super) fn open(fighter: TournamentFighter, preferred: EraFamily, what_if: bool) -> Self {
        Self::with_mode(fighter, preferred, what_if, false)
    }

    pub(super) fn open_dramatic(fighter: TournamentFighter, preferred: EraFamily, what_if: bool) -> Self {
        Self::with_mode(fighter, preferred, what_if, true)
    }

    fn with_mode(fighter: TournamentFighter, preferred: EraFamily, what_if: bool, dramatic: bool) -> Self {
        let available = listed_families(fighter, what_if);
        let families: Vec<_> = ERA_ORDER
            .into_iter()
            .filter(|family| available.contains(family)
                && (!dramatic || !Self::revisions_for(fighter, *family, what_if, true).is_empty()))
            .collect();
        let family = if families.contains(&preferred) {
            preferred
        } else {
            EraFamily::Sf1Plus
        };
        let revisions = Self::revisions_for(fighter, family, what_if, dramatic);
        let revision = *revisions
            .last()
            .expect("an available era has a flagship revision");
        let loadouts = Self::loadouts_for(revision, dramatic);
        let pick = StylePick::for_revision(revision, loadouts[0]);
        Self {
            fighter,
            what_if,
            dramatic,
            families,
            revisions,
            loadouts,
            pick,
            row: 0,
        }
    }

    pub(super) fn pick(&self) -> StylePick {
        self.pick
    }

    fn row_lengths(&self) -> [usize; 3] {
        [
            self.families.len(),
            self.revisions.len(),
            self.loadouts.len(),
        ]
    }

    pub(super) fn change(&mut self, input: MenuInput) -> bool {
        let delta = match input {
            MenuInput::Left | MenuInput::Up => -1,
            MenuInput::Right | MenuInput::Down => 1,
            _ => return false,
        };
        if matches!(input, MenuInput::Up | MenuInput::Down) {
            let lengths = self.row_lengths();
            let next = (1..3)
                .map(|step| (self.row as isize + delta * step).rem_euclid(3) as usize)
                .find(|&row| lengths[row] > 1);
            if let Some(row) = next {
                self.row = row;
                return true;
            }
            return false;
        }
        match self.row {
            0 => {
                let family = cycle(&self.families, self.pick.family, delta);
                self.revisions = Self::revisions_for(self.fighter, family, self.what_if, self.dramatic);
                self.set_revision(
                    *self
                        .revisions
                        .last()
                        .expect("an available era has a flagship revision"),
                );
            }
            1 => self.set_revision(cycle(&self.revisions, self.pick.revision, delta)),
            _ => self.pick.loadout = cycle(&self.loadouts, self.pick.loadout, delta),
        }
        true
    }

    fn revisions_for(fighter: TournamentFighter, family: EraFamily, what_if: bool, dramatic: bool) -> Vec<Revision> {
        let mut revisions = listed_revisions(fighter, family, what_if);
        if dramatic {
            revisions.retain(|&revision| DramaticRules::supports_style(revision));
        }
        revisions
    }

    fn loadouts_for(revision: Revision, dramatic: bool) -> Vec<Loadout> {
        let mut choices = loadouts(revision);
        if dramatic {
            choices.truncate(1);
        }
        choices
    }

    fn set_revision(&mut self, revision: Revision) {
        self.loadouts = Self::loadouts_for(revision, self.dramatic);
        self.pick = StylePick::for_revision(revision, self.loadouts[0]);
    }

    pub(super) fn render(
        &self,
        argb: &mut [u32],
        font: &RomFont<'_>,
        panel: (usize, usize, usize, usize),
        side_ink: u8,
    ) {
        let (x, y, w, h) = panel;
        darken(argb, x + 2, y + 2, w - 4, h - 4);
        let lengths = self.row_lengths();
        let labels = ["ERA", "REV", "LOAD"];
        let values = [
            self.pick.family.name(),
            revision_label(self.pick.revision),
            loadout_label(self.pick),
        ];
        let accent = font.pen(side_ink, 0);
        for (i, (label, value)) in labels.into_iter().zip(values).enumerate() {
            let row_y = y + 11 + 17 * i;
            let active = self.row == i;
            let color = if active {
                accent
            } else {
                font.pen(if lengths[i] > 1 { ink::BLUE } else { ink::DIM }, 0)
            };
            outline(
                argb,
                x + 5,
                row_y,
                w - 10,
                13,
                if active { color } else { font.pen(ink::DIM, 1) },
            );
            fallback_text(argb, x + 9, row_y + 4, label, color, 1);
            let value_x = x + 33;
            let value_w = w - 40;
            if lengths[i] > 1 {
                fallback_text(argb, value_x, row_y + 4, "<", color, 1);
                fallback_text(argb, value_x + value_w - ADVANCE, row_y + 4, ">", color, 1);
            }
            centered(argb, value_x, row_y + 4, value_w, value, color);
        }
        let footer = fit(&summary(self.fighter, self.pick), (w - 6) / ADVANCE);
        centered(argb, x, y + 69, w, &footer, font.pen(ink::GOLD_A, 0));
        centered(argb, x, y + 87, w, "CONFIRM / START: LOCK", accent);
    }
}

fn cycle<T: Copy + PartialEq>(options: &[T], current: T, delta: isize) -> T {
    let at = options
        .iter()
        .position(|&option| option == current)
        .expect("the selected value is available");
    options[(at as isize + delta).rem_euclid(options.len() as isize) as usize]
}

fn centered(argb: &mut [u32], x: usize, y: usize, width: usize, label: &str, color: u32) {
    let text_width = ADVANCE * label.chars().count();
    debug_assert!(text_width <= width);
    fallback_text(
        argb,
        x + width.saturating_sub(text_width) / 2,
        y,
        label,
        color,
        1,
    );
}

fn revision_label(revision: Revision) -> &'static str {
    match revision {
        Revision::Sf1Plus => "87+",
        Revision::Ww => "WW",
        Revision::Ce => "CE",
        Revision::Hf => "HF",
        Revision::Super => "SUPER",
        Revision::St => "ST",
        Revision::Sf3Ng => "NG",
        Revision::Sf3TwoI => "2I",
        Revision::Sf3ThreeS => "3S",
        Revision::Sf5 => "S1",
        Revision::Sf5Ae => "AE",
        Revision::Sf5Ce => "CE",
        Revision::Sf6 => "DRIVE",
        _ => revision.name(),
    }
}

fn loadout_label(pick: StylePick) -> &'static str {
    match (pick.revision, pick.loadout) {
        (Revision::St, Loadout::Standard) => "NEW",
        (Revision::Sf6, _) => "SA I-III",
        (_, Loadout::Art1) => "SA I",
        (_, Loadout::Art2) => "SA II",
        (_, Loadout::Art3) => "SA III",
        (_, Loadout::UltraDouble) => "ULTRA W",
        (_, Loadout::Skill1Trigger1) => "VS I / VT I",
        (_, Loadout::Skill1Trigger2) => "VS I / VT II",
        (_, Loadout::Skill2Trigger1) => "VS II / VT I",
        (_, Loadout::Skill2Trigger2) => "VS II / VT II",
        (_, loadout) => loadout.name(),
    }
}

fn summary(fighter: TournamentFighter, pick: StylePick) -> String {
    match (pick.family, pick.loadout) {
        (EraFamily::Sf1987, _) => "1987 ARCADE STYLE",
        (EraFamily::Sf1Plus, _) => "OVERHAUL STYLE",
        (EraFamily::Sf2, Loadout::Old) => return format!("O.{} (NO SUPER GAUGE)", fighter.name()),
        (EraFamily::Sf2, _) if pick.revision == Revision::St => "SUPER COMBO GAUGE",
        (EraFamily::Sf2, _) => "NO SUPER GAUGE",
        (EraFamily::SfZ, Loadout::VIsm) => "CUSTOM COMBO GAUGE",
        (EraFamily::SfZ, Loadout::XIsm) => "1-LEVEL SUPER GAUGE",
        (EraFamily::SfZ, _) => "3-LEVEL SUPER GAUGE",
        (EraFamily::Sf3, art) => {
            let (name, stocks) = style_art(fighter, pick.revision, art).unwrap_or(("SUPER ART", 3));
            let unit = if stocks == 1 { "STOCK" } else { "STOCKS" };
            let full = format!("{name} ({stocks} {unit})");
            if full.len() <= 24 {
                return full;
            }
            let short = format!("{name} ({stocks} STK)");
            return if short.len() <= 24 { short } else { name.into() };
        }
        (EraFamily::Sf4, _) => "SUPER / REVENGE GAUGES",
        (EraFamily::Sf5, _) => "EX / V-GAUGE",
        (EraFamily::Sf6, _) => "6-BAR DRIVE · SA1/2/3",
    }
    .into()
}

pub(super) fn render_badge(
    argb: &mut [u32],
    x: usize,
    y: usize,
    width: usize,
    pick: StylePick,
    color: u32,
) {
    let label = match pick.revision {
        Revision::Sf6 => "SF6 · DRIVE SYSTEM".into(),
        _ if pick.loadout != Loadout::Standard || pick.revision == Revision::St => {
            let load = match pick.loadout {
                Loadout::Skill1Trigger1 => "VS1/VT1",
                Loadout::Skill1Trigger2 => "VS1/VT2",
                Loadout::Skill2Trigger1 => "VS2/VT1",
                Loadout::Skill2Trigger2 => "VS2/VT2",
                _ => loadout_label(pick),
            };
            format!("{} · {load}", pick.revision.name())
        }
        _ => pick.revision.name().into(),
    };
    let badge_w = ADVANCE * label.chars().count() + 6;
    let badge_x = x + (width - badge_w) / 2;
    outline(argb, badge_x, y, badge_w, 9, color);
    centered(argb, badge_x, y + 2, badge_w, &label, color);
}
