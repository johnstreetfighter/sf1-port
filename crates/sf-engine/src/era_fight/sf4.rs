//! Street Fighter IV rules for the translated fighter engine.
use super::{
    EraFighterState, EraProjectileState, MotionTracker, FULL_SUB_HP, STOCK_AT, SUB_HP_PER_POINT,
};
use crate::config::RulesetPreset;
use crate::era::{EraFamily, Loadout, Revision};
use crate::fighter::Bus;
use crate::moves::{Availability, ButtonClass, Dir, MoveEntry, MoveInput, MoveKind, Recognizer};
use crate::roster::TournamentFighter;
use sf_video::era_hud::{Fill, GaugeRow, GaugeState};

use ButtonClass::*;
use Dir::*;
use TournamentFighter::{Adon, Gen, Ken, Ryu, Sagat};

pub(crate) const FOCUS_CHARGE: u8 = 0x2e;
pub(crate) const FOCUS: u8 = 0x2f;
pub(crate) const DASH: u8 = 0x1e;
pub(crate) const BACKDASH: u8 = 0x1d;
pub(crate) const REVENGE_MAX: u16 = FULL_SUB_HP * 2 / 3;
const REVENGE_READY: u16 = REVENGE_MAX / 2;
const FOCUS_LEVEL_TWO: u8 = 20;
const FOCUS_LEVEL_THREE: u8 = 50;
const DASH_WINDOW: u8 = 12;
const FOCUS_CONTACT_WINDOW: u8 = 15;
const RECOVERY_DELAY: u8 = 70;
const CRUMPLE_PAUSE: u8 = 20;
const MOTION_SLOTS: usize = 8;

const QCF: &[Dir] = &[Down, DownForward, Forward];
const QCB: &[Dir] = &[Down, DownBack, Back];
const DP: &[Dir] = &[Forward, Down, DownForward];
const HCB: &[Dir] = &[Forward, DownForward, Down, DownBack, Back];
const QCF2: &[Dir] = super::DOUBLE_QUARTER_CIRCLE;
const QCB2: &[Dir] = super::DOUBLE_QUARTER_CIRCLE_BACK;
const AIR_QCF2: &[Dir] = &[Jump, Down, DownForward, Forward, Down, DownForward, Forward];
const AIR_QCB: &[Dir] = &[Jump, Down, DownBack, Back];
const CHARGE_BF: &[Dir] = &[ChargeBack, Forward];
const CHARGE_DU: &[Dir] = &[ChargeDown, Up];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Sf4State {
    pub(crate) revenge: u16,
    motions: [MotionTracker; MOTION_SLOTS],
    recoverable_hp: u16,
    recovery_delay: u8,
    focus_frames: u8,
    focus_armor: u8,
    focus_level: u8,
    focus_contact: u8,
    focus_hit: bool,
    focus_paid: bool,
    red: bool,
    absorbed: bool,
    tap_direction: u8,
    tap_window: u8,
    previous_direction: u8,
    dash_trail: u8,
    wakeup_requested: bool,
    wakeup_used: bool,
    crumple: u8,
    ultra_scale: u8,
    active_ex: Option<u8>,
    punch_taps: u8,
    punch_window: u8,
    angry_scar: bool,
    charged_uppercut: bool,
    phase: u8,
}

crate::canonical::canonical_struct!(Sf4State, "Sf4State.v1", {
    revenge, motions, recoverable_hp, recovery_delay, focus_frames, focus_armor,
    focus_level, focus_contact, focus_hit, focus_paid, red, absorbed, tap_direction,
    tap_window, previous_direction, dash_trail, wakeup_requested, wakeup_used, crumple, ultra_scale,
    active_ex, punch_taps, punch_window, angry_scar, charged_uppercut, phase,
});

impl Sf4State {
    pub(crate) fn revenge_row(self) -> GaugeRow {
        let row = GaugeRow::bar(Fill::of(self.revenge, REVENGE_MAX), Fill::FULL);
        if self.revenge >= REVENGE_READY {
            row.with_state(GaugeState::Flashing {
                lit: self.phase & 4 == 0,
            })
        } else {
            row
        }
    }

    pub(crate) fn fill(&mut self) {
        self.revenge = REVENGE_MAX;
    }

    pub(crate) fn focus_tint(self) -> Option<(u32, u8)> {
        if self.red && (self.focus_frames != 0 || self.focus_level != 0) {
            return Some((0xffff3322, 140));
        }
        match self.focus_frames {
            0 => self.active_ex.map(|_| (0xffffcc44, 104)),
            1 => Some((0xff101016, 190)),
            2..=19 => Some((0xffffffff, 95)),
            _ => Some((0xffffe9a6, 145)),
        }
    }

    pub(crate) fn dash_trail(self) -> bool {
        self.dash_trail != 0
    }

    fn finish_focus(&mut self) {
        self.focus_frames = 0;
        self.focus_armor = 0;
        self.focus_level = 0;
        self.focus_contact = 0;
        self.focus_hit = false;
        self.focus_paid = false;
        self.red = false;
    }

    fn record_direction(&mut self, direction: u8) -> Option<u8> {
        self.tap_window = self.tap_window.saturating_sub(1);
        if self.previous_direction == direction {
            return None;
        }
        self.previous_direction = direction;
        match direction {
            1 | 2 if self.tap_window != 0 && self.tap_direction == direction => {
                self.tap_window = 0;
                Some(direction)
            }
            1 | 2 => {
                self.tap_direction = direction;
                self.tap_window = DASH_WINDOW;
                None
            }
            0 => None,
            _ => {
                self.tap_window = 0;
                None
            }
        }
    }

    fn reset_command_motions(&mut self) {
        self.motions = [MotionTracker::default(); MOTION_SLOTS];
        self.punch_taps = 0;
        self.punch_window = 0;
    }
}

pub(super) const fn vitality(fighter: TournamentFighter) -> u16 {
    match fighter {
        Gen => 864,
        Adon | TournamentFighter::Geki => 912,
        Sagat | TournamentFighter::Mike | TournamentFighter::Birdie => 1008,
        _ => FULL_SUB_HP,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cost {
    Special,
    Super,
    Ultra1,
    Ultra2,
    Stance,
    AngryScar,
}

#[derive(Clone, Copy)]
struct Command {
    slot: u8,
    name: &'static str,
    input: MoveInput,
    cost: Cost,
    ex: Option<&'static str>,
    airborne: bool,
}

impl Command {
    const fn new(
        slot: u8,
        name: &'static str,
        motion: &'static [Dir],
        button: ButtonClass,
        cost: Cost,
    ) -> Self {
        Self {
            slot,
            name,
            input: MoveInput { motion, button },
            cost,
            ex: None,
            airborne: false,
        }
    }

    const fn ex(mut self, name: &'static str) -> Self {
        self.ex = Some(name);
        self
    }

    const fn air(mut self) -> Self {
        self.airborne = true;
        self
    }

    fn motion(self) -> &'static [Dir] {
        self.input
            .motion
            .strip_prefix(&[Jump])
            .unwrap_or(self.input.motion)
    }

    fn usable(self, state: &EraFighterState) -> bool {
        match self.cost {
            Cost::Ultra1 => {
                state.super_capacity != 0
                    && matches!(state.style.loadout, Loadout::Ultra1 | Loadout::UltraDouble)
            }
            Cost::Ultra2 => {
                state.super_capacity != 0
                    && matches!(state.style.loadout, Loadout::Ultra2 | Loadout::UltraDouble)
            }
            Cost::Super => state.super_capacity != 0,
            Cost::AngryScar => state.super_capacity != 0 && state.style.revision != Revision::Sf4,
            Cost::Special | Cost::Stance => true,
        }
    }

    fn ex_button(self) -> ButtonClass {
        if self.input.button.kick() {
            PairKick
        } else {
            PairPunch
        }
    }

    fn entry(self, name: &'static str, button: ButtonClass, kind: MoveKind) -> MoveEntry {
        MoveEntry {
            name,
            kind,
            input: MoveInput {
                button,
                ..self.input
            },
            recognizer: Recognizer::FamilyMove,
            produces: self.slot,
            call_out: name,
            available: Availability {
                fighters: &[Ryu, Ken, Sagat, Adon, Gen],
                presets: &RulesetPreset::ALL,
                player_table: false,
            },
        }
    }
}

use Cost::*;

const RYU: &[Command] = &[
    Command::new(0x2a, "METSU HADOKEN", QCF2, TriplePunch, Ultra1),
    Command::new(0x2b, "METSU SHORYUKEN", QCF2, TripleKick, Ultra2),
    Command::new(0x29, "SHINKU HADOKEN", QCF2, Punch, Super),
    Command::new(0x1a, "HADOKEN", QCF, Punch, Special).ex("EX HADOKEN"),
    Command::new(0x19, "SHORYUKEN", DP, Punch, Special).ex("EX SHORYUKEN"),
    Command::new(0x1b, "TATSUMAKI SENPUKYAKU", QCB, Kick, Special).ex("EX TATSUMAKI SENPUKYAKU"),
];
const KEN: &[Command] = &[
    Command::new(0x2a, "SHINRYUKEN", QCF2, TriplePunch, Ultra1),
    Command::new(0x2b, "GUREN SENPUKYAKU", QCF2, TripleKick, Ultra2),
    Command::new(0x29, "SHORYUREPPA", QCF2, Punch, Super),
    Command::new(0x1a, "HADOKEN", QCF, Punch, Special).ex("EX HADOKEN"),
    Command::new(0x19, "SHORYUKEN", DP, Punch, Special).ex("EX SHORYUKEN"),
    Command::new(0x1b, "TATSUMAKI SENPUKYAKU", QCB, Kick, Special).ex("EX TATSUMAKI SENPUKYAKU"),
];
const SAGAT: &[Command] = &[
    Command::new(0x2a, "TIGER DESTRUCTION", QCF2, TripleKick, Ultra1),
    Command::new(0x2b, "TIGER CANNON", QCF2, TriplePunch, Ultra2),
    Command::new(0x29, "TIGER GENOCIDE", QCF2, Kick, Super),
    Command::new(0x1a, "TIGER SHOT", QCF, Punch, Special).ex("EX TIGER SHOT"),
    Command::new(0x1b, "LOW TIGER SHOT", QCF, Kick, Special).ex("EX LOW TIGER SHOT"),
    Command::new(0x1f, "TIGER UPPERCUT", DP, Punch, Special).ex("EX TIGER UPPERCUT"),
    Command::new(0x19, "TIGER KNEE CRUSH", DP, Kick, Special).ex("EX TIGER KNEE CRUSH"),
    Command::new(0x2c, "ANGRY SCAR", QCB2, Kick, AngryScar),
];
const ADON: &[Command] = &[
    Command::new(0x2a, "JAGUAR REVOLVER", QCB2, TripleKick, Ultra1),
    Command::new(0x2b, "JAGUAR AVALANCHE", QCF2, TripleKick, Ultra2),
    Command::new(0x29, "JAGUAR VARIED ASSAULT", QCF2, Punch, Super),
    Command::new(0x19, "RISING JAGUAR", DP, Kick, Special).ex("EX RISING JAGUAR"),
    Command::new(0x1b, "JAGUAR KICK", QCB, Kick, Special).ex("EX JAGUAR KICK"),
    Command::new(0x20, "JAGUAR TOOTH", HCB, Kick, Special).ex("EX JAGUAR TOOTH"),
    Command::new(0x21, "AIR JAGUAR KICK", AIR_QCB, Kick, Special)
        .ex("EX AIR JAGUAR KICK")
        .air(),
];
const GEN_MANTIS: &[Command] = &[
    Command::new(0x2a, "ZETSUEI", QCF2, TriplePunch, Ultra1),
    Command::new(0x2c, "SHITENKETSU", QCB2, TriplePunch, Ultra2),
    Command::new(0x29, "ZAN'EI", QCF2, Punch, Super),
    Command::new(0x1a, "HYAKURENKO", &[], MashPunch, Special),
    Command::new(0x1b, "GEKIRO", DP, Kick, Special).ex("EX GEKIRO"),
    Command::new(0x1f, "CRANE STANCE", &[], TripleKick, Stance),
];
const GEN_CRANE: &[Command] = &[
    Command::new(0x2d, "RYUKOHA", QCF2, TripleKick, Ultra1),
    Command::new(0x1c, "TEIGA", AIR_QCF2, TripleKick, Ultra2).air(),
    Command::new(0x2b, "JYAKOHA", QCF2, Kick, Super),
    Command::new(0x20, "JASEN", CHARGE_BF, Punch, Special).ex("EX JASEN"),
    Command::new(0x21, "OGA", CHARGE_DU, Kick, Special).ex("EX OGA"),
    Command::new(0x1f, "MANTIS STANCE", &[], TriplePunch, Stance),
];

impl EraFighterState {
    pub(crate) fn is_sf4(&self) -> bool {
        self.style.family == EraFamily::Sf4
            && (self.what_if_active()
                || match self.fighter {
                    Some(Ryu | Ken | Sagat | Gen) => true,
                    Some(Adon) => self.style.revision != Revision::Sf4,
                    _ => false,
                })
    }

    fn sf4_commands(&self) -> &'static [Command] {
        if !self.is_sf4() || self.what_if_active() {
            return &[];
        }
        match self.fighter {
            Some(Ryu) => RYU,
            Some(Ken) => KEN,
            Some(Sagat) => SAGAT,
            Some(Adon) => ADON,
            Some(Gen) if self.gen_crane => GEN_CRANE,
            Some(Gen) => GEN_MANTIS,
            _ => &[],
        }
    }

    pub(crate) fn sf4_active_moves(&self) -> Vec<MoveEntry> {
        if !self.is_sf4() {
            return Vec::new();
        }
        let mut entries = Vec::new();
        for &command in self.sf4_commands().iter().filter(|c| c.usable(self)) {
            let kind = match command.cost {
                Ultra1 | Ultra2 | Super => MoveKind::Super,
                Stance => MoveKind::System,
                AngryScar | Special => MoveKind::Special,
            };
            entries.push(command.entry(command.name, command.input.button, kind));
            if let Some(ex) = command.ex.filter(|_| self.super_capacity != 0) {
                entries.push(command.entry(ex, command.ex_button(), MoveKind::Special));
            }
        }
        if self.super_capacity == 0 {
            return entries;
        }
        for (name, button) in [("FOCUS ATTACK", MediumPair), ("RED FOCUS ATTACK", RedFocus)] {
            if button != RedFocus
                || (self.style.revision == Revision::Usf4 && self.super_capacity != 0)
            {
                entries.push(Command::new(FOCUS, name, &[], button, Special).entry(
                    name,
                    button,
                    MoveKind::System,
                ));
            }
        }
        entries
    }

    pub(crate) fn sf4_progress(&self, slot: u8) -> (u8, u8) {
        self.sf4_commands()
            .iter()
            .enumerate()
            .find(|(_, command)| command.slot == slot)
            .map_or((0, 0), |(i, command)| {
                let tracker = self.sf4.motions[i];
                let jump = u8::from(command.airborne && tracker.position != 0);
                (tracker.position + jump, tracker.remaining)
            })
    }

    pub(crate) fn sf4_airborne_move(&self, slot: u8) -> bool {
        self.is_sf4()
            && self.alpha_move_active(slot)
            && self
                .sf4_commands()
                .iter()
                .any(|c| c.slot == slot && c.airborne)
    }

    pub(crate) fn sf4_special_active(&self, slot: u8) -> bool {
        self.is_sf4()
            && ((self.what_if_special_active(slot) && !self.what_if_airborne(slot))
                || self
                    .sf4_commands()
                    .iter()
                    .any(|c| c.slot == slot && c.cost == Special && !c.airborne))
    }

    pub(crate) fn sf4_tint(&self) -> Option<(u32, u8)> {
        self.is_sf4().then(|| self.sf4.focus_tint()).flatten()
    }

    pub(crate) fn sf4_dash_trail(&self) -> bool {
        self.is_sf4() && self.sf4.dash_trail()
    }

    pub(crate) fn sf4_revenge_points(&self) -> Option<u16> {
        self.is_sf4().then_some(self.sf4.revenge)
    }

    pub(crate) fn set_sf4_practice_revenge(&mut self, value: Option<u16>) {
        if self.is_sf4() {
            self.sf4.revenge = value.unwrap_or(REVENGE_MAX).min(REVENGE_MAX);
        }
    }

    pub(super) fn sf4_spend_ultra(&mut self, disable_comeback_scaling: bool) {
        let revenge = if self.unified_gauge {
            REVENGE_MAX
        } else {
            self.sf4.revenge
        };
        let mut scale = 90 + (40 * u32::from(revenge) / u32::from(REVENGE_MAX)) as u8;
        if self.style.loadout == Loadout::UltraDouble && !disable_comeback_scaling {
            scale = (u16::from(scale) * 75 / 100) as u8;
        }
        self.sf4.revenge = 0;
        self.sf4.ultra_scale = scale;
    }

    pub(crate) fn sf4_projectile_scale(&self, slot: u8) -> u8 {
        if !self.is_sf4() || !self.alpha_move_active(slot) {
            0
        } else if self.sf4.ultra_scale != 0 {
            self.sf4.ultra_scale
        } else if self.sf4.active_ex == Some(slot) {
            120
        } else {
            0
        }
    }
}

impl Bus<'_> {
    pub(crate) fn era_sf4_input(
        &mut self,
        ps: u32,
        entity: u16,
        relative: u8,
        actionable: bool,
        special_active: bool,
        hard_knockdown: bool,
    ) -> bool {
        let Some(side) = self
            .era_side(ps)
            .filter(|&side| self.toggle_state.era.fighters[side].is_sf4())
        else {
            return false;
        };
        let (held, fresh) = self.era_buttons(ps);
        let move_id = self.eb(entity, 0xa);
        let state = &mut self.toggle_state.era.fighters[side];

        let commands = state.sf4_commands();
        for (i, command) in commands.iter().enumerate() {
            state.sf4.motions[i].step(relative, command.motion());
        }
        state.sf4.punch_window = state.sf4.punch_window.saturating_sub(1);
        if state.sf4.punch_window == 0 {
            state.sf4.punch_taps = 0;
        }
        if fresh & super::PUNCH_MASK != 0 {
            state.sf4.punch_taps = state.sf4.punch_taps.saturating_add(1);
            state.sf4.punch_window = 24;
        }
        if state.fighter == Some(Gen) && move_id == 0x1b && fresh & super::KICK_MASK != 0 {
            state.repeated_kick = true;
        }
        if state.super_capacity == 0 {
            state.sf4.finish_focus();
            return false;
        }
        let dash = state.sf4.record_direction(relative);
        if state.style.revision == Revision::Usf4
            && hard_knockdown
            && !state.sf4.wakeup_used
            && fresh != 0
            && held.count_ones() >= 2
        {
            state.sf4.wakeup_requested = true;
        }
        if state.sf4.crumple != 0 {
            return true;
        }
        if actionable {
            state.sf4.wakeup_requested = false;
            state.sf4.wakeup_used = false;
        }
        if move_id != FOCUS_CHARGE && move_id != FOCUS {
            state.sf4.finish_focus();
        }

        let fadc_cost = if state.unified_gauge {
            STOCK_AT
        } else {
            2 * STOCK_AT
        };
        let charging = state.sf4.focus_frames != 0;
        let after_contact = move_id == FOCUS && state.sf4.focus_contact != 0;
        if let Some(direction) =
            dash.filter(|_| charging || after_contact || actionable && fresh == 0)
        {
            let focus_cancel = charging || after_contact;
            let paid = state.sf4.focus_paid || state.sf4.focus_hit && state.sf4.focus_level >= 2;
            let cost = if focus_cancel && !paid { fadc_cost } else { 0 };
            if state.super_meter >= cost {
                return self.era_sf4_dash(ps, entity, direction, cost, focus_cancel);
            }
        }
        if charging {
            let held = MediumPair.held(held);
            let state = &mut self.toggle_state.era.fighters[side].sf4;
            state.focus_frames += u8::from(held);
            if held && state.focus_frames < FOCUS_LEVEL_THREE {
                return true;
            }
            return self.era_sf4_release_focus(ps, entity);
        }

        let red = state.style.revision == Revision::Usf4 && RedFocus.held(held);
        let focus = if red { RedFocus } else { MediumPair };
        let rising_chord = focus.held(held) && !focus.held(held & !fresh);
        if rising_chord && (actionable || special_active) {
            let cost = match (red, special_active && !actionable) {
                (true, true) => 3 * STOCK_AT,
                (false, true) => fadc_cost,
                (true, false) => 2 * STOCK_AT,
                _ => 0,
            };
            if state.super_meter >= cost {
                return self.era_sf4_begin_focus(ps, entity, cost, red);
            }
        }
        false
    }

    fn era_sf4_dash(
        &mut self,
        ps: u32,
        entity: u16,
        direction: u8,
        cost: u8,
        focus_cancel: bool,
    ) -> bool {
        let slot = if direction == 1 { DASH } else { BACKDASH };
        if !self.era_start_banked_move(ps, entity, slot, 0, None) {
            return false;
        }
        let side = self.era_side(ps).expect("SF4 dash has a fighter state");
        let state = &mut self.toggle_state.era.fighters[side];
        state.super_meter -= cost;
        state.sf4.finish_focus();
        state.sf4.dash_trail = if focus_cancel { 12 } else { 0 };
        true
    }

    fn era_sf4_begin_focus(&mut self, ps: u32, entity: u16, cost: u8, red: bool) -> bool {
        if !self.era_start_banked_move(ps, entity, FOCUS_CHARGE, 0, None) {
            return false;
        }
        let side = self.era_side(ps).expect("Focus has a fighter state");
        let state = &mut self.toggle_state.era.fighters[side];
        state.super_meter -= cost;
        state.sf4.focus_frames = 1;
        state.sf4.focus_paid = cost != 0;
        state.sf4.focus_armor = if red { u8::MAX } else { 1 };
        state.sf4.red = red;
        state.sf4.tap_window = 0;
        true
    }

    fn era_sf4_release_focus(&mut self, ps: u32, entity: u16) -> bool {
        let side = self.era_side(ps).expect("Focus has a fighter state");
        let frames = self.toggle_state.era.fighters[side].sf4.focus_frames;
        let level = match frames {
            FOCUS_LEVEL_THREE.. => 3,
            FOCUS_LEVEL_TWO.. => 2,
            _ => 1,
        };
        if !self.era_start_banked_move(ps, entity, FOCUS, 0, None) {
            self.toggle_state.era.fighters[side].sf4.finish_focus();
            return false;
        }
        let state = &mut self.toggle_state.era.fighters[side].sf4;
        state.focus_frames = 0;
        state.focus_level = level;
        if level < 3 && !state.red {
            state.focus_armor = 0;
        }
        true
    }

    pub(crate) fn era_sf4_command(
        &mut self,
        ps: u32,
        entity: u16,
        airborne: bool,
        canceling: bool,
    ) -> bool {
        let Some(side) = self.era_side(ps) else {
            return false;
        };
        let (held, fresh) = self.era_buttons(ps);
        let state = &self.toggle_state.era.fighters[side];
        let meter = state.super_meter;
        let revenge = if state.unified_gauge {
            REVENGE_MAX
        } else {
            state.sf4.revenge
        };
        let unified = state.unified_gauge;
        let intent = state
            .sf4_commands()
            .iter()
            .enumerate()
            .find_map(|(i, command)| {
                if !command.usable(state)
                    || airborne != command.airborne
                    || canceling && command.cost == Stance
                {
                    return None;
                }
                let motion = command.motion();
                if !motion.is_empty() && !state.sf4.motions[i].ready(motion) {
                    return None;
                }
                let fresh_chord =
                    |button: ButtonClass| button.held(held) && !button.held(held & !fresh);
                let ex =
                    command.ex.is_some() && meter >= STOCK_AT && fresh_chord(command.ex_button());
                let button_fired = match command.input.button {
                    Punch if command.cost == Super => {
                        fresh & super::PUNCH_MASK != 0 && !TriplePunch.held(held)
                    }
                    Kick if command.cost == Super => {
                        fresh & super::KICK_MASK != 0 && !TripleKick.held(held)
                    }
                    Punch => fresh & super::PUNCH_MASK != 0,
                    Kick => fresh & super::KICK_MASK != 0,
                    MashPunch => state.sf4.punch_taps >= 4 && fresh & super::PUNCH_MASK != 0,
                    chord => fresh_chord(chord),
                };
                if !ex && !button_fired {
                    return None;
                }
                let cost = match command.cost {
                    Special if ex => STOCK_AT,
                    AngryScar => STOCK_AT,
                    Super if unified => 3 * STOCK_AT,
                    Super => 4 * STOCK_AT,
                    Ultra1 | Ultra2 if unified => 3 * STOCK_AT,
                    Ultra1 | Ultra2 if revenge < REVENGE_READY => return None,
                    _ => 0,
                };
                (meter >= cost).then_some((*command, ex, cost))
            });
        let Some((command, ex, cost)) = intent else {
            return false;
        };
        let ultra = matches!(command.cost, Ultra1 | Ultra2);
        let super_name = (ultra || command.cost == Super).then_some(command.name);
        let level = if super_name.is_some() {
            3
        } else {
            u8::from(ex)
        };
        let air_y = airborne.then(|| self.center_y(entity));
        if !self.era_start_banked_move(ps, entity, command.slot, level, super_name) {
            return false;
        }
        if let Some(y) = air_y {
            self.era_adjust_air_start(ps, entity, y);
        }
        let state = &mut self.toggle_state.era.fighters[side];
        state.super_meter -= cost;
        state.sf4.active_ex = ex.then_some(command.slot);
        state.sf4.reset_command_motions();
        state.repeated_kick = false;
        if ultra {
            state.sf4_spend_ultra(self.era_rules.disable_comeback_scaling);
        } else if command.cost == Stance {
            state.gen_crane = !state.gen_crane;
        } else if command.cost == AngryScar {
            state.sf4.angry_scar = true;
        } else if state.fighter == Some(Sagat) && command.slot == 0x1f {
            state.sf4.charged_uppercut = std::mem::take(&mut state.sf4.angry_scar);
        }
        // The native streams use a 1..=3 strength, not the input latch encoding.
        let attack_buttons = if command.input.button.kick() {
            held >> 3
        } else {
            held
        };
        let strength = if ex || attack_buttons & 4 != 0 {
            3
        } else if attack_buttons & 2 != 0 {
            2
        } else {
            1
        };
        self.set_eb(entity, 0x34, strength);
        if command.cost == Special && !ex {
            self.era_meter_gain(ps, 4);
        }
        true
    }

    pub(crate) fn era_sf4_started(&mut self, ps: u32, move_id: u8) {
        let Some(side) = self
            .era_side(ps)
            .filter(|&side| self.toggle_state.era.fighters[side].is_sf4())
        else {
            return;
        };
        let state = &mut self.toggle_state.era.fighters[side];
        state.sf4.active_ex = None;
        state.sf4.ultra_scale = 0;
        state.sf4.charged_uppercut = false;
        if move_id != FOCUS_CHARGE && move_id != FOCUS {
            state.sf4.finish_focus();
        }
        if move_id != DASH && move_id != BACKDASH {
            state.sf4.dash_trail = 0;
        }
        if !state.alpha_move_active(move_id) && state.sf4_special_active(move_id) {
            self.era_meter_gain(ps, 4);
        }
    }

    pub(crate) fn era_sf4_focus_unblockable(&self, owner: u32, move_id: u8) -> bool {
        move_id == FOCUS
            && self.era_state(owner).is_some_and(|state| {
                state.is_sf4() && state.alpha_move_active(move_id) && state.sf4.focus_level == 3
            })
    }

    pub(crate) fn era_sf4_blocked(&mut self, owner: u32, move_id: u8) {
        let Some(side) = self.era_side(owner) else {
            return;
        };
        let state = &mut self.toggle_state.era.fighters[side];
        if state.is_sf4() && move_id == FOCUS && state.alpha_move_active(move_id) {
            state.sf4.focus_contact = FOCUS_CONTACT_WINDOW;
            state.sf4.focus_hit = false;
            state.sf4.focus_armor = 0;
        }
    }

    pub(crate) fn era_sf4_crumple_hit(&mut self, owner: u32, target: u32, move_id: u8) -> bool {
        if move_id != FOCUS
            || !self.era_state(owner).is_some_and(|state| {
                state.is_sf4()
                    && state.alpha_move_active(FOCUS)
                    && (state.sf4.focus_level >= 2 || state.sf4.red)
                    && state.sf4.focus_hit
            })
        {
            return false;
        }
        let Some(side) = self.era_side(target) else {
            return false;
        };
        self.toggle_state.era.fighters[side].sf4.crumple = CRUMPLE_PAUSE;
        true
    }

    pub(crate) fn era_sf4_holding_crumple(&self, ps: u32) -> bool {
        self.era_state(ps)
            .is_some_and(|state| state.sf4.crumple != 0)
    }

    pub(crate) fn era_sf4_wakeup_extra(&mut self, ps: u32) -> u8 {
        let Some(side) = self.era_side(ps) else {
            return 0;
        };
        let state = &mut self.toggle_state.era.fighters[side];
        if state.is_sf4()
            && state.super_capacity != 0
            && state.style.revision == Revision::Usf4
            && std::mem::take(&mut state.sf4.wakeup_requested)
        {
            state.sf4.wakeup_used = true;
            11
        } else {
            0
        }
    }

    pub(crate) fn era_sf4_take_absorbed(&mut self, target: u32) -> bool {
        self.era_side(target).is_some_and(|side| {
            std::mem::take(&mut self.toggle_state.era.fighters[side].sf4.absorbed)
        })
    }

    /// Adjust a contact's sub-HP damage and suppress reaction on Focus armor.
    pub(crate) fn era_sf4_damage(
        &mut self,
        owner: u32,
        target: u32,
        move_id: u8,
        mut damage: u16,
        is_chip: bool,
        projectile: Option<EraProjectileState>,
    ) -> Option<u16> {
        let attacking = self
            .era_side(owner)
            .filter(|&i| self.toggle_state.era.fighters[i].is_sf4());
        let defending = self.era_side(target);
        let mut armor_break = false;
        if let Some(side) = attacking {
            let state = &self.toggle_state.era.fighters[side];
            let focus = move_id == FOCUS && state.alpha_move_active(FOCUS);
            armor_break = focus && (state.sf4.focus_level >= 2 || state.sf4.red)
                || state
                    .sf4_commands()
                    .iter()
                    .any(|c| c.slot == move_id && matches!(c.cost, Ultra1 | Ultra2));
            if !is_chip {
                if focus {
                    damage = match state.sf4.focus_level {
                        2 => damage * 135 / 100,
                        3 => damage * 175 / 100,
                        _ => damage,
                    };
                    if state.sf4.red {
                        damage = damage * 3 / 2;
                    }
                } else if projectile.is_some()
                    && matches!(
                        (state.fighter, move_id),
                        (Some(Ryu), 0x29 | 0x2a) | (Some(Sagat), 0x2b)
                    )
                {
                    damage = 58;
                }
                let scale = projectile.map_or_else(
                    || state.sf4_projectile_scale(move_id),
                    |shot| shot.sf4_scale,
                );
                if scale != 0 {
                    damage = (u32::from(damage) * u32::from(scale) / 100) as u16;
                }
                if state.sf4.charged_uppercut && move_id == 0x1f && projectile.is_none() {
                    damage = damage * 3 / 2;
                }
            }
        }
        if let Some(side) = defending {
            let disable_revenge = self.era_rules.disable_comeback_scaling;
            let state = &mut self.toggle_state.era.fighters[side];
            state.sf4.absorbed = false;
            if !is_chip {
                let sf4 = state.is_sf4();
                if sf4 && !disable_revenge && !state.unified_gauge && state.super_capacity != 0 {
                    let gain = if state.sf4.red && state.sf4.focus_armor != 0 && !armor_break {
                        damage.saturating_mul(2)
                    } else {
                        damage
                    };
                    state.sf4.revenge = state.sf4.revenge.saturating_add(gain).min(REVENGE_MAX);
                }
                if sf4 && state.sf4.focus_armor != 0 && !armor_break {
                    if state.sf4.focus_armor != u8::MAX {
                        state.sf4.focus_armor -= 1;
                    }
                    let lost = damage.min(state.sub_hp.saturating_sub(1));
                    state.sub_hp -= lost;
                    state.sf4.recoverable_hp += lost;
                    state.sf4.recovery_delay = RECOVERY_DELAY;
                    state.sf4.absorbed = true;
                    return None;
                }
                state.sf4.recoverable_hp = 0;
                state.sf4.crumple = 0;
                state.sf4.active_ex = None;
                state.sf4.ultra_scale = 0;
                state.sf4.charged_uppercut = false;
                state.sf4.finish_focus();
                if sf4 {
                    self.era_meter_gain(target, (damage / (SUB_HP_PER_POINT * 2)).min(10) as u8);
                }
            }
        }
        if let Some(side) = attacking.filter(|_| !is_chip) {
            let state = &mut self.toggle_state.era.fighters[side];
            if move_id == FOCUS && state.alpha_move_active(FOCUS) {
                state.sf4.focus_contact = FOCUS_CONTACT_WINDOW;
                state.sf4.focus_hit = true;
                state.sf4.focus_armor = 0;
            }
        }
        Some(damage)
    }

    pub(crate) fn era_sf4_tick(&mut self, ps: u32) {
        let Some(side) = self.era_side(ps) else {
            return;
        };
        let state = &mut self.toggle_state.era.fighters[side];
        state.sf4.crumple = state.sf4.crumple.saturating_sub(1);
        if !state.is_sf4() {
            return;
        }
        let sf4 = &mut state.sf4;
        sf4.phase = sf4.phase.wrapping_add(1);
        sf4.dash_trail = sf4.dash_trail.saturating_sub(1);
        sf4.focus_contact = sf4.focus_contact.saturating_sub(1);
        sf4.recovery_delay = sf4.recovery_delay.saturating_sub(1);
        if sf4.recoverable_hp != 0 && sf4.recovery_delay == 0 && state.sub_hp != 0 {
            let restored = sf4.recoverable_hp.min(3);
            sf4.recoverable_hp -= restored;
            state.sub_hp = state.sub_hp.saturating_add(restored).min(state.max_sub_hp);
        }
    }
}
