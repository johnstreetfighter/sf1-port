//! Street Fighter V's meters, defensive windows and input priority on the 1987 engine.
use super::{Bus, EraFighterState, EvilRyuKit, MotionTracker, DOUBLE_QUARTER_CIRCLE, STOCK_AT};
use crate::config::RulesetPreset;
use crate::data_bank::{
    KAGE_ASHURA_AIR_BACK, KAGE_ASHURA_AIR_FORWARD, KAGE_ASHURA_BACK, KAGE_ASHURA_FORWARD,
    KAGE_HADOKEN, KAGE_KUREKIJIN, KAGE_MISOGI, KAGE_RYUSOKYAKU, KAGE_SEKIEIKEN, KAGE_SHAKUNETSU,
};
use crate::era::{EraFamily, GaugeMode, Loadout, Revision, StylePick};
use crate::moves::{Availability, ButtonClass, Dir, MoveEntry, MoveInput, MoveKind, Recognizer};
use crate::roster::TournamentFighter::{self, Birdie, Ken, Ryu, Sagat};
use sf_video::callouts::CalloutKind;
use sf_video::era_hud::{Fill, GaugeRow, GaugeState};

pub(crate) const UPPERCUT: u8 = 0x1f;
pub(crate) const EX_FORWARD: u8 = 0x20;
pub(crate) const EX_RISE: u8 = 0x21;
pub(crate) const EX_KICK: u8 = 0x22;
pub(crate) const EX_LOW_SHOT: u8 = 0x23;
pub(crate) const TRIGGER_ATTACK: u8 = 0x24;
pub(crate) const TRIGGER_FORWARD: u8 = 0x25;
pub(crate) const COUNTER_STRIKE: u8 = 0x26;
pub(crate) const CRITICAL_ART: u8 = 0x29;
pub(crate) const TRIGGER: u8 = 0x2a;
pub(crate) const SKILL_ONE: u8 = 0x2b;
pub(crate) const SKILL_TWO: u8 = 0x2c;
pub(crate) const REVERSAL: u8 = 0x2d;
pub(crate) const SHIFT: u8 = 0x2e;
pub(crate) const SHIFT_BREAK: u8 = 0x2f;

const LP: u8 = 1 << 0;
const MP: u8 = 1 << 1;
const HP: u8 = 1 << 2;
const LK: u8 = 1 << 3;
const MK: u8 = 1 << 4;
const HK: u8 = 1 << 5;
const PUNCHES: u8 = LP | MP | HP;
const KICKS: u8 = LK | MK | HK;
const QCF: &[Dir] = &[Dir::Down, Dir::DownForward, Dir::Forward];
const QCB: &[Dir] = &[Dir::Down, Dir::DownBack, Dir::Back];
const DP: &[Dir] = &[Dir::Forward, Dir::Down, Dir::DownForward];
const FORWARD: &[Dir] = &[Dir::Forward];
const BACK: &[Dir] = &[Dir::Back];
const HALF_FORWARD: &[Dir] = &[Dir::Back, Dir::DownBack, Dir::Down, Dir::DownForward, Dir::Forward];
const HALF_BACK: &[Dir] = &[Dir::Forward, Dir::DownForward, Dir::Down, Dir::DownBack, Dir::Back];
const FIGHTERS: &[TournamentFighter] = &TournamentFighter::ALL;
const SHIFT_FRAMES: u8 = 10;
const SHIFT_BREAK_FRAMES: u8 = 18;
const COUNTER_FRAMES: u8 = 12;
const WHITE_RECOVERY_DELAY: u8 = 60;
const KAGE_TRIGGER_DURATION: u16 = 800;
const KAGE_ASHURA_COST: u16 = 250;
const KAGE_RIKUDO_DURATION: u16 = 900;
const KAGE_SEKIEIKEN_V_GAIN: u8 = 20;
const SENHA_CHARGE_TICKS: u8 = 18;
const SENHA_ARMOR_TICK: u8 = 3;
const KAGE_SHORYUKEN: u8 = 0x19;

pub(crate) fn available(style: StylePick, fighter: Option<TournamentFighter>) -> bool {
    style.family == EraFamily::Sf5
        && match fighter {
            Some(Ryu | Ken | Birdie) => true,
            Some(Sagat) => matches!(style.revision, Revision::Sf5Ae | Revision::Sf5Ce),
            Some(_) => style.revision == Revision::Sf5Ce,
            None => false,
        }
}

fn canonical(fighter: TournamentFighter) -> bool {
    matches!(fighter, Ryu | Ken | Birdie | Sagat)
}

fn second_skill(loadout: Loadout) -> bool {
    matches!(loadout, Loadout::Skill2Trigger1 | Loadout::Skill2Trigger2)
}

fn second_trigger(loadout: Loadout) -> bool {
    matches!(loadout, Loadout::Skill1Trigger2 | Loadout::Skill2Trigger2)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KageVState {
    senha_hold: Option<u8>,
    senha_armor: bool,
    senha_vt_charged: bool,
    absorbed: bool,
    vt2_confirm: Option<u8>,
    half_forward: MotionTracker,
    half_back: MotionTracker,
}

crate::canonical::canonical_struct!(KageVState, "KageVState.v1", {
    senha_hold, senha_armor, senha_vt_charged, absorbed, vt2_confirm, half_forward, half_back,
});

impl KageVState {
    fn step(&mut self, direction: u8) {
        self.half_forward.step(direction, HALF_FORWARD);
        self.half_back.step(direction, HALF_BACK);
    }

    fn clear_motions(&mut self) {
        self.half_forward = MotionTracker::default();
        self.half_back = MotionTracker::default();
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct VSystem {
    capacity: u8,
    meter: u8,
    trigger_left: u16,
    trigger_total: u16,
    shift_left: u8,
    break_left: u8,
    slow_left: u8,
    reversal_left: u8,
    counter_left: u8,
    counter_returns_strike: bool,
    skill_charge: u8,
    charged_move: u8,
    white_life: u16,
    white_delay: u8,
    critical: MotionTracker,
    quarter_forward: MotionTracker,
    quarter_back: MotionTracker,
    rising: MotionTracker,
}

crate::canonical::canonical_struct!(VSystem, "Sf5VSystem.v1", {
    capacity, meter, trigger_left, trigger_total, shift_left, break_left, slow_left,
    reversal_left, counter_left, counter_returns_strike, skill_charge, charged_move,
    white_life, white_delay, critical, quarter_forward, quarter_back, rising,
});

impl VSystem {
    pub(super) fn share_meter(&mut self, source: Self) {
        self.meter = source.meter;
    }

    pub(super) fn apply_satsui_surge(&mut self, previous: Self) {
        self.meter = previous.meter;
        self.gain(self.capacity / 2);
    }

    pub(crate) fn configure(
        &mut self,
        style: StylePick,
        fighter: Option<TournamentFighter>,
        mode: GaugeMode,
    ) {
        if mode == GaugeMode::NoGauges || !available(style, fighter) {
            *self = Self::default();
            return;
        }
        self.capacity = STOCK_AT * if second_trigger(style.loadout) { 3 } else { 2 };
        self.meter = self.meter.min(self.capacity);
    }

    pub(super) fn gauge_row(&self, show_trigger_timer: bool) -> Option<GaugeRow> {
        if self.capacity == 0 {
            return None;
        }
        let draining = show_trigger_timer && self.trigger_left != 0;
        let value = if draining {
            (u32::from(self.capacity) * u32::from(self.trigger_left))
                .div_ceil(u32::from(self.trigger_total)) as u8
        } else {
            self.meter
        };
        let row = GaugeRow::stock(
            self.capacity / STOCK_AT,
            value / STOCK_AT,
            Fill::of(u16::from(value % STOCK_AT), u16::from(STOCK_AT)),
        );
        Some(if draining {
            row.with_state(GaugeState::Draining)
        } else {
            row
        })
    }

    pub(crate) fn tick(&mut self, fill: bool, ready: bool, hp: &mut u16, max_hp: u16) {
        self.trigger_left = self.trigger_left.saturating_sub(1);
        self.shift_left = self.shift_left.saturating_sub(1);
        self.break_left = self.break_left.saturating_sub(1);
        self.reversal_left = self.reversal_left.saturating_sub(1);
        self.counter_left = self.counter_left.saturating_sub(1);
        self.white_delay = self.white_delay.saturating_sub(1);
        if self.white_delay == 0 && self.white_life != 0 && *hp != 0 {
            let heal = self.white_life.min(2).min(max_hp.saturating_sub(*hp));
            *hp += heal;
            self.white_life -= heal;
            if *hp == max_hp {
                self.white_life = 0;
            }
        }
        if fill && ready && self.trigger_left == 0 && self.shift_left == 0 && self.break_left == 0 {
            self.meter = self.capacity;
        }
    }

    fn step(&mut self, direction: u8) {
        self.critical.step(direction, DOUBLE_QUARTER_CIRCLE);
        self.quarter_forward.step(direction, QCF);
        self.quarter_back.step(direction, QCB);
        self.rising.step(direction, DP);
    }

    fn clear_motions(&mut self) {
        self.critical = MotionTracker::default();
        self.quarter_forward = MotionTracker::default();
        self.quarter_back = MotionTracker::default();
        self.rising = MotionTracker::default();
    }

    fn gain(&mut self, amount: u8) {
        if self.trigger_left == 0 {
            self.meter = self.meter.saturating_add(amount).min(self.capacity);
        }
    }

    fn add_white(&mut self, amount: u16) {
        self.white_life = self.white_life.saturating_add(amount);
        self.white_delay = WHITE_RECOVERY_DELAY;
    }

    fn clear_white(&mut self) {
        self.white_life = 0;
        self.white_delay = 0;
    }

    pub(crate) fn warm(&self) -> bool {
        self.trigger_left != 0
    }

    pub(crate) fn evading(&self) -> bool {
        self.shift_left != 0 || self.break_left != 0
    }
}

#[derive(Clone, Copy)]
enum Action {
    CriticalArt,
    Enhanced(u8),
    Special(u8),
    Skill(u8),
    Trigger(u8),
    TriggerFollowUp(u8),
    Reversal,
    Shift,
    ShiftBreak,
}

impl Action {
    fn slot(self) -> u8 {
        match self {
            Self::CriticalArt => CRITICAL_ART,
            Self::Enhanced(slot)
            | Self::Special(slot)
            | Self::Skill(slot)
            | Self::TriggerFollowUp(slot) => slot,
            Self::Trigger(slot) => slot,
            Self::Reversal => REVERSAL,
            Self::Shift => SHIFT,
            Self::ShiftBreak => SHIFT_BREAK,
        }
    }

    fn level(self, fighter: TournamentFighter) -> u8 {
        match self {
            Self::CriticalArt => 3,
            Self::Enhanced(_) | Self::Trigger(_) => 1,
            Self::TriggerFollowUp(_) if fighter == Ken => 3,
            Self::TriggerFollowUp(COUNTER_STRIKE | TRIGGER_FORWARD) if fighter == Sagat => 1,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy)]
struct Special {
    slot: u8,
    name: &'static str,
    motion: &'static [Dir],
    kick: bool,
}

const SHOTO_EX: &[Special] = &[
    Special {
        slot: EX_RISE,
        name: "EX SHORYUKEN",
        motion: DP,
        kick: false,
    },
    Special {
        slot: EX_FORWARD,
        name: "EX HADOKEN",
        motion: QCF,
        kick: false,
    },
    Special {
        slot: EX_KICK,
        name: "EX TATSUMAKI SENPUKYAKU",
        motion: QCB,
        kick: true,
    },
];
const KAGE_SPECIALS: &[Special] = &[
    Special { slot: KAGE_SHORYUKEN, name: "SHORYUKEN", motion: DP, kick: false },
    Special { slot: KAGE_HADOKEN, name: "HADOKEN", motion: QCF, kick: false },
    Special { slot: KAGE_SHAKUNETSU, name: "SHAKUNETSU HADOKEN", motion: HALF_BACK, kick: false },
    Special { slot: KAGE_KUREKIJIN, name: "KUREKIJIN", motion: QCB, kick: true },
    Special { slot: KAGE_RYUSOKYAKU, name: "RYUSOKYAKU", motion: HALF_FORWARD, kick: true },
];
const KAGE_EX: &[Special] = &[
    SHOTO_EX[0],
    SHOTO_EX[1],
    Special { name: "EX KUREKIJIN", ..SHOTO_EX[2] },
];
const BIRDIE_EX: &[Special] = &[
    Special {
        slot: EX_FORWARD,
        name: "EX BULL HEAD",
        motion: QCF,
        kick: false,
    },
    Special {
        slot: EX_KICK,
        name: "EX HANGING CHAIN",
        motion: QCF,
        kick: true,
    },
];
const SAGAT_EX: &[Special] = &[
    Special {
        slot: EX_RISE,
        name: "EX TIGER UPPERCUT",
        motion: DP,
        kick: false,
    },
    Special {
        slot: EX_KICK,
        name: "EX TIGER KNEE CRUSH",
        motion: DP,
        kick: true,
    },
    Special {
        slot: EX_FORWARD,
        name: "EX TIGER SHOT",
        motion: QCF,
        kick: false,
    },
    Special {
        slot: EX_LOW_SHOT,
        name: "EX GRAND TIGER SHOT",
        motion: QCF,
        kick: true,
    },
];

fn ex_specials(fighter: TournamentFighter) -> &'static [Special] {
    match fighter {
        Ryu | Ken => SHOTO_EX,
        Birdie => BIRDIE_EX,
        Sagat => SAGAT_EX,
        _ => &[],
    }
}

fn critical(fighter: TournamentFighter) -> (&'static str, ButtonClass) {
    match fighter {
        Ryu => ("SHINKU HADOKEN", ButtonClass::Punch),
        Ken => ("GUREN ENJINKYAKU", ButtonClass::Kick),
        Birdie => ("SKIP TO MY CHAIN", ButtonClass::Punch),
        _ => ("TIGER RAMPAGE", ButtonClass::Kick),
    }
}

fn skill(fighter: TournamentFighter, loadout: Loadout) -> (u8, &'static str) {
    let second = second_skill(loadout);
    if !canonical(fighter) {
        if let Some(kit) = super::what_if::kit(fighter, EraFamily::Sf5, false) {
            return (
                if second { SKILL_TWO } else { SKILL_ONE },
                kit.v_skills[usize::from(second)],
            );
        }
    }
    let name = match (fighter, second) {
        (Ryu, false) => "MIND'S EYE",
        (Ryu, true) => "THRUST STRIKE",
        (Ken, false) => "QUICK STEP",
        (Ken, true) => "RYUSENKYAKU",
        (Birdie, false) => "BREAK TIME",
        (Birdie, true) => "CHEWING TIME",
        (Sagat, false) => "ANGRY CHARGE",
        (Sagat, true) => "HANUMAN CHARGE",
        (_, false) => "V-SKILL I",
        (_, true) => "V-SKILL II",
    };
    (if second { SKILL_TWO } else { SKILL_ONE }, name)
}

fn trigger(fighter: TournamentFighter, loadout: Loadout) -> (&'static str, u16) {
    if !canonical(fighter) {
        if let Some(kit) = super::what_if::kit(fighter, EraFamily::Sf5, false) {
            return (kit.v_triggers[usize::from(second_trigger(loadout))], 720);
        }
    }
    match (fighter, second_trigger(loadout)) {
        (Ryu, false) => ("DENJIN RENKI", 1000),
        (Ryu, true) => ("KAKKO FUBATSU", 1500),
        (Ken, false) => ("HEAT RUSH", 800),
        (Ken, true) => ("SHINRYUKEN", 540),
        (Birdie, false) => ("ENJOY TIME", 900),
        (Birdie, true) => ("BIRDIE TIME", 800),
        (Sagat, false) => ("TIGER CHARGE", 720),
        (Sagat, true) => ("TIGER ASSAULT", 720),
        (_, false) => ("V-TRIGGER I", 720),
        (_, true) => ("V-TRIGGER II", 720),
    }
}

fn trigger_follow_up(
    fighter: TournamentFighter,
    loadout: Loadout,
    forward: bool,
) -> Option<(u8, &'static str)> {
    match (fighter, second_trigger(loadout), forward) {
        (Ryu, true, _) => Some((TRIGGER_ATTACK, "ISSHIN")),
        (Ken, true, _) => Some((TRIGGER_ATTACK, "SHINRYUKEN")),
        (Birdie, true, true) => Some((TRIGGER_FORWARD, "BULL CAPTURE")),
        (Birdie, true, false) => Some((TRIGGER_ATTACK, "BULL SWING")),
        (Sagat, true, true) => Some((TRIGGER_FORWARD, "TIGER RUSH")),
        (Sagat, true, false) => Some((TRIGGER_ATTACK, "TIGER SPIKE")),
        (Sagat, false, _) => Some((COUNTER_STRIKE, "TIGER CANNON")),
        _ => None,
    }
}

fn trigger_slot(fighter: TournamentFighter, loadout: Loadout) -> u8 {
    if fighter == Ken && second_trigger(loadout) {
        TRIGGER_ATTACK
    } else {
        TRIGGER
    }
}

fn reversal(fighter: TournamentFighter) -> &'static str {
    match fighter {
        Ryu => "HASHOGEKI",
        Ken => "SENPU NATAOTOSHI",
        Birdie => "PEPPER POT",
        Sagat => "TIGER REVENGE",
        _ => "V-REVERSAL",
    }
}

fn shift_break(fighter: TournamentFighter) -> &'static str {
    match fighter {
        Ryu => "FUMIKOMI JODAN KAIRIGERI",
        Ken => "SHADOW THUNDER-KICK",
        Birdie => "BULL CRUSH",
        Sagat => "TIGER TORNADO",
        _ => "V-SHIFT BREAK",
    }
}

impl VSystem {
    fn motion(&self, pattern: &[Dir]) -> MotionTracker {
        match pattern {
            [Dir::Down, Dir::DownForward, Dir::Forward] => self.quarter_forward,
            [Dir::Down, Dir::DownBack, Dir::Back] => self.quarter_back,
            [Dir::Forward, Dir::Down, Dir::DownForward] => self.rising,
            _ => self.critical,
        }
    }
}

impl EraFighterState {
    fn sf5_ex_specials(&self, fighter: TournamentFighter) -> &'static [Special] {
        if self.is_kage() { KAGE_EX } else { ex_specials(fighter) }
    }

    fn sf5_critical(&self, fighter: TournamentFighter) -> (&'static str, ButtonClass) {
        if self.is_kage() { ("METSU SHORYUKEN", ButtonClass::Punch) } else { critical(fighter) }
    }

    fn sf5_skill(&self, fighter: TournamentFighter) -> (u8, &'static str) {
        match (self.is_kage(), second_skill(self.style.loadout)) {
            (true, true) => (KAGE_SEKIEIKEN, "SEKIEIKEN"),
            (true, false) => (SKILL_ONE, "SENHA KASSATSU"),
            _ => skill(fighter, self.style.loadout),
        }
    }

    fn sf5_trigger(&self, fighter: TournamentFighter) -> (&'static str, u16) {
        if self.sf5_kage_rikudo() {
            ("RIKUDO OSATSU", KAGE_RIKUDO_DURATION)
        } else if self.is_kage() {
            ("TAIGYAKU MUDO", KAGE_TRIGGER_DURATION)
        } else {
            trigger(fighter, self.style.loadout)
        }
    }

    fn sf5_kage_rikudo(&self) -> bool {
        self.is_kage() && second_trigger(self.style.loadout)
    }

    pub(super) fn sf5_spend_kage_demon(&mut self) -> bool {
        if !self.sf5_kage_rikudo() || !self.sf5.warm() || self.super_meter < 3 * STOCK_AT {
            return false;
        }
        self.super_meter -= 3 * STOCK_AT;
        self.sf5.trigger_left = 0;
        self.sf5.clear_motions();
        if let EvilRyuKit::Kage { v } = &mut self.evil_ryu_kit {
            v.clear_motions();
            v.vt2_confirm = None;
        }
        true
    }

    fn sf5_record_rikudo_contact(&mut self, slot: u8, guarded: bool) {
        if !self.sf5_kage_rikudo()
            || (guarded && self.sf5.warm() && matches!(slot, KAGE_SHORYUKEN | EX_RISE))
        {
            return;
        }
        let grounded_normal = matches!(slot, 0x0d..=0x18);
        let authored_special = self.alpha_move_active(slot) && matches!(slot,
            KAGE_SHORYUKEN | KAGE_HADOKEN | KAGE_KUREKIJIN | KAGE_SHAKUNETSU | KAGE_RYUSOKYAKU
                | EX_FORWARD | EX_RISE | EX_KICK | KAGE_SEKIEIKEN
        );
        if grounded_normal || authored_special {
            if let EvilRyuKit::Kage { v } = &mut self.evil_ryu_kit {
                v.vt2_confirm = Some(slot);
            }
        }
    }

    fn sf5_rikudo_confirmed(&self, slot: u8) -> bool {
        matches!(self.evil_ryu_kit, EvilRyuKit::Kage { v } if v.vt2_confirm == Some(slot))
            && (!self.sf5.warm() || slot != KAGE_SEKIEIKEN)
    }

    fn sf5_motion(&self, pattern: &[Dir]) -> MotionTracker {
        if let EvilRyuKit::Kage { v } = self.evil_ryu_kit {
            if pattern == HALF_FORWARD {
                return v.half_forward;
            }
            if pattern == HALF_BACK {
                return v.half_back;
            }
        }
        self.sf5.motion(pattern)
    }

    pub(crate) fn sf5_keep_senha_hold(&mut self, buttons: u8) {
        if let EvilRyuKit::Kage { v } = &mut self.evil_ryu_kit {
            if v.senha_hold.is_none() || !ButtonClass::MediumPair.held(buttons) {
                v.senha_hold = None;
                v.senha_armor = false;
            }
        }
    }

    pub(crate) fn sf5_vt_charged_senha(&self) -> bool {
        self.alpha_move_active(SKILL_ONE)
            && matches!(self.evil_ryu_kit, EvilRyuKit::Kage { v } if v.senha_vt_charged)
    }

    pub(crate) fn sf5_active_moves(&self) -> Vec<MoveEntry> {
        if !available(self.style, self.fighter) {
            return Vec::new();
        }
        let fighter = self.fighter.expect("available SF5 fighter");
        let move_entry = |name, slot, kind, motion, button| MoveEntry {
            name,
            produces: slot,
            kind,
            input: MoveInput { motion, button },
            call_out: name,
            recognizer: Recognizer::FamilyMove,
            available: Availability {
                fighters: FIGHTERS,
                presets: &RulesetPreset::ALL,
                player_table: !fighter.is_shoto(),
            },
        };
        let mut entries = Vec::new();
        if self.is_kage() {
            entries.extend(KAGE_SPECIALS.iter().map(|special| {
                let button = if special.kick { ButtonClass::Kick } else { ButtonClass::Punch };
                move_entry(special.name, special.slot, MoveKind::Special, special.motion, button)
            }));
        }
        if fighter == Sagat {
            entries.push(move_entry(
                "TIGER UPPERCUT",
                UPPERCUT,
                MoveKind::Special,
                DP,
                ButtonClass::Punch,
            ));
        }
        if self.sf5.capacity == 0 {
            return entries;
        }
        for special in self.sf5_ex_specials(fighter) {
            let button = if special.kick {
                ButtonClass::PairKick
            } else {
                ButtonClass::PairPunch
            };
            entries.push(move_entry(
                special.name,
                special.slot,
                MoveKind::Special,
                special.motion,
                button,
            ));
        }
        if canonical(fighter) && !self.what_if_active() {
            let (name, button) = self.sf5_critical(fighter);
            entries.push(move_entry(
                name,
                CRITICAL_ART,
                MoveKind::Super,
                DOUBLE_QUARTER_CIRCLE,
                button,
            ));
        }

        let (slot, name) = self.sf5_skill(fighter);
        entries.push(move_entry(
            name,
            slot,
            MoveKind::System,
            &[],
            ButtonClass::MediumPair,
        ));
        let (name, _) = self.sf5_trigger(fighter);
        entries.push(move_entry(
            name,
            trigger_slot(fighter, self.style.loadout),
            MoveKind::System,
            &[],
            ButtonClass::HeavyPair,
        ));
        if self.sf5_kage_rikudo() {
            entries.push(move_entry("MISOGI", KAGE_MISOGI, MoveKind::System, &[], ButtonClass::HeavyPair));
        } else if self.is_kage() {
            for (name, slot, motion) in [
                ("ASHURA SENKU FORWARD", KAGE_ASHURA_FORWARD, FORWARD),
                ("ASHURA SENKU BACKWARD", KAGE_ASHURA_BACK, BACK),
            ] {
                entries.push(move_entry(name, slot, MoveKind::System, motion, ButtonClass::HeavyPair));
            }
        } else {
            if let Some((slot, name)) = trigger_follow_up(fighter, self.style.loadout, false)
                .filter(|(slot, _)| *slot != trigger_slot(fighter, self.style.loadout))
            {
                entries.push(move_entry(name, slot, MoveKind::System, &[], ButtonClass::HeavyPair));
            }
            if let Some((slot, name)) = trigger_follow_up(fighter, self.style.loadout, true)
                .filter(|(slot, _)| *slot == TRIGGER_FORWARD)
            {
                entries.push(move_entry(name, slot, MoveKind::System, FORWARD, ButtonClass::HeavyPair));
            }
        }
        entries.push(move_entry(
            if self.is_kage() { "SENBUGEKI" } else { reversal(fighter) },
            REVERSAL,
            MoveKind::System,
            FORWARD,
            if fighter == Ken {
                ButtonClass::TripleKick
            } else {
                ButtonClass::TriplePunch
            },
        ));
        if self.style.revision == Revision::Sf5Ce {
            entries.push(move_entry(
                "V-SHIFT",
                SHIFT,
                MoveKind::System,
                &[],
                ButtonClass::ShiftPair,
            ));
            entries.push(move_entry(
                if self.is_kage() { "YASHAZUKI" } else { shift_break(fighter) },
                SHIFT_BREAK,
                MoveKind::System,
                &[],
                ButtonClass::ShiftPair,
            ));
        }
        entries
    }

    pub(crate) fn sf5_progress(&self, slot: u8) -> (u8, u8) {
        let Some(fighter) = self.fighter else {
            return (0, 0);
        };
        let tracker = if slot == CRITICAL_ART {
            self.sf5.critical
        } else if slot == UPPERCUT && !self.is_kage() {
            self.sf5.rising
        } else if let Some(special) = self.sf5_ex_specials(fighter).iter()
            .chain(KAGE_SPECIALS.iter().filter(|_| self.is_kage()))
            .find(|special| special.slot == slot)
        {
            self.sf5_motion(special.motion)
        } else {
            return (0, 0);
        };
        (tracker.position, tracker.remaining)
    }

    pub(crate) fn sf5_practice_points(&self) -> Option<(u8, u16, u16)> {
        (self.sf5.capacity != 0).then_some((
            self.sf5.meter,
            self.sf5.trigger_left,
            self.sf5.trigger_total,
        ))
    }

    pub(crate) fn set_sf5_practice_points(&mut self, saved: (u8, u16, u16)) {
        (
            self.sf5.meter,
            self.sf5.trigger_left,
            self.sf5.trigger_total,
        ) = saved;
    }

    pub(crate) fn fill_sf5_practice(&mut self, slot: u8) {
        let Some(fighter) = self.fighter else { return };
        if slot == trigger_slot(fighter, self.style.loadout) {
            // SHOW and TRY lend the original meter, so a completed activation
            // can be repeated as soon as its own move recovers.
            self.sf5.trigger_left = 0;
            self.sf5.trigger_total = 0;
            self.sf5.meter = self.sf5.capacity;
        } else if matches!(slot, TRIGGER_ATTACK | TRIGGER_FORWARD | COUNTER_STRIKE)
            || self.is_kage() && slot == KAGE_ASHURA_AIR_BACK
            || self.sf5_kage_rikudo() && slot == KAGE_MISOGI
        {
            let (_, duration) = self.sf5_trigger(fighter);
            self.sf5.trigger_total = duration;
            self.sf5.trigger_left = duration;
        } else if self.sf5.trigger_left == 0 {
            self.sf5.meter = self.sf5.capacity;
        }
    }

    pub(crate) fn sf5_tint(&self) -> Option<sf_video::compositor::PaletteTint> {
        use sf_video::compositor::PaletteTint;
        if self.sf5.evading() {
            Some(PaletteTint {
                color: 0xff46_dfff,
                strength: 112,
            })
        } else if let EvilRyuKit::Kage { v } = self.evil_ryu_kit {
            if v.senha_hold.is_some_and(|ticks| ticks >= SENHA_ARMOR_TICK) {
                Some(PaletteTint { color: 0xffdd_483c, strength: 105 })
            } else {
                let color = if self.sf5_kage_rikudo() { 0xffbb_3527 } else { 0xffae_366e };
                self.sf5.warm().then_some(PaletteTint { color, strength: 65 })
            }
        } else if self.sf5.warm() {
            Some(PaletteTint {
                color: 0xffff_7426,
                strength: 75,
            })
        } else {
            None
        }
    }

    pub(crate) fn sf5_afterimages(&self) -> bool {
        self.sf5.evading()
            || (self.is_kage() && matches!(self.active_alpha_slot,
                KAGE_ASHURA_FORWARD | KAGE_ASHURA_BACK | KAGE_ASHURA_AIR_FORWARD | KAGE_ASHURA_AIR_BACK))
    }

    pub(crate) fn v_trigger_cost(&self, mode: GaugeMode) -> u8 {
        if mode == GaugeMode::Unified {
            2 * STOCK_AT
        } else {
            self.sf5.capacity
        }
    }

    fn award_v_meter(&mut self, amount: u8) {
        let surge = self.world_tour_perk(crate::world_tour::native::Perk::MeterSurge);
        let gain = super::world_tour::meter_gain(u16::from(amount), surge, &mut self.tour.v_gain_remainder);
        self.sf5.gain(gain.min(u16::from(u8::MAX)) as u8);
    }

    fn can_spend_v(&self, mode: GaugeMode, amount: u8) -> bool {
        let amount = self.meter_cost(amount);
        self.sf5.capacity != 0
            && self.sf5.trigger_left == 0
            && if mode == GaugeMode::Unified {
                self.super_meter >= amount
            } else {
                self.sf5.meter >= amount
            }
    }

    fn spend_v(&mut self, mode: GaugeMode, amount: u8) {
        let amount = self.meter_cost(amount);
        if mode == GaugeMode::Unified {
            self.super_meter -= amount;
        } else {
            self.sf5.meter -= amount;
        }
    }
}

fn chord(held: u8, pressed: u8, mask: u8) -> bool {
    held & mask == mask && pressed & mask != 0
}

fn pair(held: u8, pressed: u8, mask: u8) -> bool {
    (held & mask).count_ones() >= 2 && pressed & mask != 0
}

impl Bus<'_> {
    pub(crate) fn sf5_kage_senha_branch(&mut self, ps: u32) -> Option<u8> {
        let Some(side) = self.era_side(ps) else { return Some(0) };
        let state = &mut self.toggle_state.era.fighters[side];
        let EvilRyuKit::Kage { v } = &mut state.evil_ryu_kit else { return Some(0) };
        let mut charged = false;
        if let Some(ticks) = &mut v.senha_hold {
            *ticks += 1;
            if *ticks == SENHA_ARMOR_TICK {
                v.senha_armor = true;
            }
            if *ticks < SENHA_CHARGE_TICKS {
                return None;
            }
            charged = true;
            v.senha_hold = None;
        }
        let triggered = state.sf5.warm();
        v.senha_vt_charged = triggered && charged;
        // The ROM-bank continuation table orders tap, charged, and then the
        // same two releases during Taigyaku Mudo.
        Some(2 * u8::from(triggered) + u8::from(charged))
    }

    pub(crate) fn sf5_input(
        &mut self,
        ps: u32,
        entity: u16,
        relative: u8,
        free_ground: bool,
        cancel_ground: bool,
        blockstun: bool,
        in_jump: bool,
    ) -> bool {
        let Some(side) = self.era_side(ps) else {
            return false;
        };
        let state = &self.toggle_state.era.fighters[side];
        let Some(fighter) = state
            .fighter
            .filter(|_| available(state.style, state.fighter))
        else {
            return false;
        };
        let (held, pressed) = self.era_buttons(ps);
        let move_id = self.eb(entity, 0xa);
        let posture = self.eb(entity, 9);
        let special_cancel = if state.sf5_kage_rikudo() {
            posture & 0x80 != 0 && state.sf5_rikudo_confirmed(move_id)
        } else {
            posture & 0xc1 == 0xc0
                && self.eb(entity, 0x14) == 0
                && (matches!(move_id, 0x19..=0x1b)
                    || (state.alpha_move_active(move_id)
                        && (matches!(move_id, UPPERCUT | EX_FORWARD..=EX_LOW_SHOT)
                            || state.is_kage() && move_id == KAGE_RYUSOKYAKU)))
        };
        let mode = self.era_gauge_mode(side);
        let state = &mut self.toggle_state.era.fighters[side];
        state.sf5.step(relative);
        if let EvilRyuKit::Kage { v } = &mut state.evil_ryu_kit {
            v.step(relative);
        }
        let Some(action) = sf5_action(
            state,
            fighter,
            mode,
            held,
            pressed,
            relative,
            free_ground,
            cancel_ground,
            special_cancel,
            blockstun,
            in_jump,
        ) else {
            return false;
        };
        let kage = state.is_kage();
        let call_out = matches!(action, Action::CriticalArt).then(|| state.sf5_critical(fighter).0);
        let trigger_name = (kage && matches!(action, Action::Trigger(_)))
            .then(|| state.sf5_trigger(fighter).0);
        if !self.era_start_banked_move(ps, entity, action.slot(), action.level(fighter), call_out) {
            return false;
        }
        if let Action::Special(slot) = action {
            if kage {
                let buttons = if matches!(slot, KAGE_KUREKIJIN | KAGE_RYUSOKYAKU) {
                    pressed >> 3
                } else {
                    pressed
                };
                // Attack strength selects the ROM animation without marking
                // the special as a meter-spending Super in banked move state.
                let strength = if buttons & HP != 0 { 3 } else if buttons & MP != 0 { 2 } else { 1 };
                self.set_eb(entity, 0x34, strength);
            }
        }

        let state = &mut self.toggle_state.era.fighters[side];
        state.sf5.clear_motions();
        if let EvilRyuKit::Kage { v } = &mut state.evil_ryu_kit {
            v.clear_motions();
        }
        match action {
            Action::CriticalArt => state.super_meter -= state.meter_cost(3 * STOCK_AT),
            Action::Enhanced(_) => state.super_meter -= state.meter_cost(STOCK_AT),
            Action::Reversal => {
                state.spend_v(mode, STOCK_AT);
                state.sf5.reversal_left = 13;
            }
            Action::Shift => {
                state.spend_v(mode, STOCK_AT);
                state.sf5.shift_left = SHIFT_FRAMES;
            }
            Action::ShiftBreak => {
                state.sf5.shift_left = 0;
                state.sf5.break_left = 0;
            }

            Action::Trigger(_) => {
                let (_, duration) = state.sf5_trigger(fighter);
                state.spend_v(mode, state.v_trigger_cost(mode));
                state.sf5.trigger_left = duration;
                state.sf5.trigger_total = duration;
                self.toggle_state.era.freeze = self.toggle_state.era.freeze.max(15);
            }
            Action::TriggerFollowUp(KAGE_MISOGI) if state.sf5_kage_rikudo() => {
                state.sf5.trigger_left = state.sf5.trigger_left.saturating_sub(state.sf5.trigger_total / 2);
            }
            Action::TriggerFollowUp(_) if state.is_kage() => {
                state.sf5.trigger_left = state.sf5.trigger_left.saturating_sub(KAGE_ASHURA_COST);
            }
            Action::TriggerFollowUp(_) if fighter == Ryu => {
                state.sf5.counter_left = COUNTER_FRAMES;
                state.sf5.counter_returns_strike = true;
                state.sf5.trigger_left = state.sf5.trigger_left.saturating_sub(150);
            }
            Action::TriggerFollowUp(_) => {
                state.sf5.trigger_left = state
                    .sf5
                    .trigger_left
                    .saturating_sub(state.sf5.trigger_total / 2);
            }
            Action::Skill(SKILL_ONE) if state.is_kage() => {
                if let EvilRyuKit::Kage { v } = &mut state.evil_ryu_kit {
                    v.senha_hold = Some(0);
                    v.senha_armor = false;
                }
            }
            Action::Skill(SKILL_ONE) if fighter == Ryu => {
                state.sf5.counter_left = COUNTER_FRAMES;
                state.sf5.counter_returns_strike = false;
            }
            Action::Skill(SKILL_TWO) if fighter == Ryu => {
                state.sf5.counter_left = 6;
                state.sf5.counter_returns_strike = true;
            }
            Action::Skill(slot) if fighter == Sagat => {
                if state.sf5.skill_charge == 0 {
                    state.award_v_meter(12);
                }
                state.sf5.skill_charge = if slot == SKILL_ONE { UPPERCUT } else { 0x19 };
            }
            Action::Skill(SKILL_ONE) if fighter == Birdie => state.award_v_meter(16),
            _ => {}
        }
        if let Some(word) = trigger_name {
            self.era_emit_callout(ps, CalloutKind::Move { word });
        }
        self.era_share_gauge(side);
        true
    }

    /// The caller has already found a real collision. Probe invulnerability
    /// before resolving damage so misses cannot refund a shift or land a parry.
    pub(crate) fn sf5_defend(&mut self, ps: u32, entity: u16, throw_only: bool) -> bool {
        let Some(side) = self.era_side(ps) else {
            return false;
        };
        let mode = self.era_gauge_mode(side);
        let trigger_counter = self.eb(entity, 0xa) == TRIGGER_ATTACK;
        let state = &mut self.toggle_state.era.fighters[side];
        if state.sf5.shift_left != 0 {
            if !throw_only && state.sf5.break_left == 0 {
                state.sf5.break_left = SHIFT_BREAK_FRAMES;
                state.sf5.slow_left = 14;
                if mode == GaugeMode::Unified {
                    state.award_super_meter(STOCK_AT / 2);
                } else {
                    state.award_v_meter(STOCK_AT / 2);
                }
                self.era_share_gauge(side);
            }
            return true;
        }
        if throw_only {
            return false;
        }
        if state.sf5.reversal_left != 0 {
            return true;
        }
        if state.sf5.counter_left == 0 {
            return false;
        }
        state.sf5.counter_left = 0;
        let strike = state.sf5.counter_returns_strike;
        if state.sf5.warm() && !strike {
            state.sf5.trigger_left = (state.sf5.trigger_left + 50).min(state.sf5.trigger_total);
        } else {
            state.award_v_meter(20);
        }
        if strike {
            if trigger_counter {
                state.sf5.trigger_left = 0;
            }
            self.era_start_banked_move(ps, entity, COUNTER_STRIKE, 0, None);
        }
        self.era_share_gauge(side);
        self.toggle_state.era.freeze = self.toggle_state.era.freeze.max(5);
        true
    }

    pub(crate) fn sf5_throw_invulnerable(&self, ps: u32) -> bool {
        self.era_state(ps)
            .is_some_and(|state| state.sf5.shift_left != 0)
    }

    pub(crate) fn sf5_slow_tick(&mut self) -> bool {
        let mut freeze = false;
        for state in &mut self.toggle_state.era.fighters {
            if state.sf5.slow_left != 0 {
                freeze |= state.sf5.slow_left % 2 == 0;
                state.sf5.slow_left -= 1;
            }
        }
        freeze
    }

    pub(crate) fn sf5_on_move_started(&mut self, ps: u32, slot: u8) {
        if let Some(side) = self.era_side(ps) {
            let state = &mut self.toggle_state.era.fighters[side];
            if let EvilRyuKit::Kage { v } = &mut state.evil_ryu_kit {
                v.senha_vt_charged = false;
                v.vt2_confirm = None;
                if slot != SKILL_ONE {
                    v.senha_hold = None;
                    v.senha_armor = false;
                }
            }
            state.sf5.charged_move = 0;
            let basic = match slot {
                EX_RISE => UPPERCUT,
                EX_KICK => 0x19,
                _ => slot,
            };
            if state.sf5.skill_charge != 0 && basic == state.sf5.skill_charge {
                state.sf5.skill_charge = 0;
                state.sf5.charged_move = slot;
            }
        }
    }

    pub(crate) fn sf5_strike(
        &mut self,
        attacker: u16,
        defender: u16,
        slot: u8,
        chip: bool,
    ) -> bool {
        if chip {
            return false;
        }
        let ps = self.el(attacker, 0x38);
        let Some(side) = self.era_side(ps) else {
            return false;
        };
        let strength = crate::combo::level(self.eb(attacker, 0x34));
        let busy_attack = self.eb(defender, 9) & 0x80 != 0
            && matches!(self.eb(defender, 0xa), 0x0d..=0x26 | CRITICAL_ART);
        let state = &mut self.toggle_state.era.fighters[side];
        if state.sf5.capacity == 0 {
            return false;
        }
        if slot == KAGE_SEKIEIKEN && state.is_kage() && state.alpha_move_active(slot) {
            state.award_v_meter(KAGE_SEKIEIKEN_V_GAIN);
        } else if slot == SKILL_ONE || (!state.is_kage() && matches!(slot, SKILL_TWO | COUNTER_STRIKE)) {
            state.award_v_meter(20);
        }
        let crush_move = matches!(slot, 0x16 | 0x18)
            || (state.fighter == Some(Ryu) && slot == 0x10)
            || (state.fighter == Some(Ken) && matches!(slot, 0x1c..=0x1e) && strength == 3);
        if !crush_move || !busy_attack {
            self.era_share_gauge(side);
            return false;
        }
        self.toggle_state.era.fighters[side].award_v_meter(14);
        self.era_share_gauge(side);
        self.toggle_state.era.freeze = self.toggle_state.era.freeze.max(6);
        self.era_emit_callout(ps, CalloutKind::CrushCounter);
        true
    }
}

fn sf5_action(
    state: &EraFighterState,
    fighter: TournamentFighter,
    mode: GaugeMode,
    held: u8,
    pressed: u8,
    relative: u8,
    free_ground: bool,
    cancel_ground: bool,
    special_cancel: bool,
    blockstun: bool,
    in_jump: bool,
) -> Option<Action> {
    let gauge = state.sf5.capacity != 0;
    if blockstun {
        return (relative == Dir::Forward.bits()
            && (chord(held, pressed, PUNCHES) || chord(held, pressed, KICKS))
            && state.can_spend_v(mode, STOCK_AT))
        .then_some(Action::Reversal);
    }

    let shifting = held & (HP | MK) == HP | MK;
    if shifting && state.sf5.break_left != 0 {
        return Some(Action::ShiftBreak);
    }
    if state.is_kage() && state.sf5.warm() && chord(held, pressed, HP | HK) {
        if state.sf5_kage_rikudo() {
            return (free_ground || special_cancel).then_some(Action::TriggerFollowUp(KAGE_MISOGI));
        }
        if free_ground || cancel_ground || special_cancel || in_jump {
            let back = relative & (Dir::Forward.bits() | Dir::Back.bits()) == Dir::Back.bits();
            let slot = match (in_jump, back) {
                (false, false) => KAGE_ASHURA_FORWARD,
                (false, true) => KAGE_ASHURA_BACK,
                (true, false) => KAGE_ASHURA_AIR_FORWARD,
                (true, true) => KAGE_ASHURA_AIR_BACK,
            };
            return Some(Action::TriggerFollowUp(slot));
        }
    }
    if !free_ground && !cancel_ground && !special_cancel {
        return None;
    }
    if gauge && chord(held, pressed, HP | HK) {
        if state.sf5.warm() {
            if let Some((slot, _)) = trigger_follow_up(
                fighter,
                state.style.loadout,
                relative == Dir::Forward.bits(),
            ) {
                return Some(Action::TriggerFollowUp(slot));
            }
        } else if state.can_spend_v(mode, state.v_trigger_cost(mode)) {
            return Some(Action::Trigger(trigger_slot(fighter, state.style.loadout)));
        }
    }
    if free_ground
        && state.style.revision == Revision::Sf5Ce
        && relative & !3 == 0
        && chord(held, pressed, HP | MK)
        && state.can_spend_v(mode, STOCK_AT)
    {
        return Some(Action::Shift);
    }
    if gauge && (free_ground || cancel_ground) && chord(held, pressed, MP | MK) {
        return Some(Action::Skill(state.sf5_skill(fighter).0));
    }
    let (_, critical_button) = state.sf5_critical(fighter);
    let family = if critical_button.kick() {
        KICKS
    } else {
        PUNCHES
    };
    if gauge
        && canonical(fighter)
        && !state.what_if_active()
        && state.super_meter >= state.meter_cost(3 * STOCK_AT)
        && pressed & family != 0
        && state.sf5.critical.ready(DOUBLE_QUARTER_CIRCLE)
    {
        return Some(Action::CriticalArt);
    }
    if !free_ground && !cancel_ground {
        return None;
    }
    if gauge && !state.what_if_active() && state.super_meter >= state.meter_cost(STOCK_AT) {
        for special in state.sf5_ex_specials(fighter) {
            if pair(held, pressed, if special.kick { KICKS } else { PUNCHES })
                && state.sf5_motion(special.motion).ready(special.motion)
            {
                return Some(Action::Enhanced(special.slot));
            }
        }
    }
    if state.is_kage() {
        for special in KAGE_SPECIALS {
            let buttons = if special.kick { KICKS } else { PUNCHES };
            if pressed & buttons != 0
                && state.sf5_motion(special.motion).ready(special.motion)
            {
                return Some(Action::Special(special.slot));
            }
        }
    }
    if fighter == Sagat && pressed & PUNCHES != 0 && state.sf5.rising.ready(DP) {
        return Some(Action::Special(UPPERCUT));
    }
    None
}

impl Bus<'_> {
    pub(crate) fn sf5_damage(
        &mut self,
        attacker: u32,
        defender: u32,
        slot: u8,
        mut damage: u16,
        chip: bool,
    ) -> u16 {
        let owner = self.era_side(attacker);
        let target = self.era_side(defender);
        let comeback = !self.era_rules.disable_comeback_scaling;
        let demon_slam = slot == crate::combo::throw::THROW_MOVE && self.toggle_state.raging_demon.active;
        let mut recoverable = false;
        let mut armor_break = false;
        if let Some(owner) = owner {
            let state = &mut self.toggle_state.era.fighters[owner];
            if state.sf5.capacity != 0 {
                state.sf5_record_rikudo_contact(slot, chip);
                let authored = state.alpha_move_active(slot);
                armor_break = authored && slot == CRITICAL_ART;
                if !chip && state.fighter == Some(Ryu) && !state.is_kage() && slot == CRITICAL_ART {
                    damage = 72;
                }
                recoverable = authored && slot == REVERSAL;
                let powered = state.sf5.charged_move == slot
                    || (state.sf5.warm()
                        && !state.is_kage()
                        && (!second_trigger(state.style.loadout) || state.what_if_active())
                        && matches!(slot, 0x19..=0x1b | UPPERCUT | CRITICAL_ART)
                        && state.fighter != Some(Sagat));
                if powered && !chip {
                    damage = damage.saturating_add(damage / 4);
                }
                let kage_meterless = state.is_kage()
                    && (demon_slam || authored && slot == KAGE_SEKIEIKEN);
                if !chip
                    && !recoverable
                    && !kage_meterless
                    && !(authored && matches!(slot, EX_FORWARD..=EX_LOW_SHOT | CRITICAL_ART))
                {
                    state.award_super_meter(5);
                }
            }
            self.era_share_gauge(owner);
        }
        damage = self.world_tour_damage(attacker, defender, damage, chip);
        if let Some(target) = target {
            let mode = self.era_gauge_mode(target);
            let state = &mut self.toggle_state.era.fighters[target];
            let mut armored = false;
            if !chip && !armor_break && state.alpha_move_active(SKILL_ONE) {
                if let EvilRyuKit::Kage { v } = &mut state.evil_ryu_kit {
                    armored = std::mem::take(&mut v.senha_armor);
                    v.absorbed = armored;
                }
            }
            if recoverable || armored {
                damage = damage.min(state.sub_hp.saturating_sub(1));
                state.sf5.add_white(damage);
            } else if !chip {
                state.sf5.clear_white();
            }
            if state.sf5.capacity != 0 && comeback && mode != GaugeMode::Unified {
                state.award_v_meter((damage / 10).min(24) as u8);
                self.era_share_gauge(target);
            }
        }
        damage
    }

    pub(crate) fn sf5_take_absorbed(&mut self, ps: u32) -> bool {
        let Some(side) = self.era_side(ps) else { return false };
        let state = &mut self.toggle_state.era.fighters[side];
        if let EvilRyuKit::Kage { v } = &mut state.evil_ryu_kit {
            std::mem::take(&mut v.absorbed)
        } else {
            false
        }
    }

    /// Medium and heavy blocked normals add recoverable White Life. Chip
    /// from specials keeps its ordinary health and KO rules.
    pub(crate) fn sf5_block(&mut self, attacker: u16, defender: u16) {
        let atk_ps = self.el(attacker, 0x38);
        let def_ps = self.el(defender, 0x38);
        let slot = self.eb(attacker, 0xa);
        if let Some(side) = self.era_side(atk_ps) {
            let state = &mut self.toggle_state.era.fighters[side];
            state.sf5_record_rikudo_contact(slot, true);
            if slot == KAGE_SEKIEIKEN && state.is_kage() && state.alpha_move_active(slot) {
                state.sf5.gain(KAGE_SEKIEIKEN_V_GAIN / 2);
                self.era_share_gauge(side);
            }
        }
        let Some(attacking) = self.era_state(atk_ps) else {
            return;
        };
        if attacking.sf5.capacity == 0
            || attacking.alpha_move_active(slot)
            || !matches!(slot, 0x0d..=0x18 | 0x1c..=0x1e | 0x20..=0x26)
        {
            return;
        }
        let class = crate::combo::class_of(slot, crate::combo::level(self.eb(attacker, 0x34)));
        let amount = match class {
            crate::combo::Class::Medium | crate::combo::Class::Jump(2) => 10,
            crate::combo::Class::Heavy
            | crate::combo::Class::Sweep
            | crate::combo::Class::Jump(3) => 18,
            _ => 0,
        };
        let Some(side) = self.era_side(def_ps) else {
            return;
        };
        let amount = self.world_tour_damage(atk_ps, def_ps, amount, true);
        let mode = self.era_gauge_mode(side);
        let state = &mut self.toggle_state.era.fighters[side];
        let amount = amount.min(state.sub_hp.saturating_sub(1));
        if amount == 0 {
            return;
        }
        state.sf5.add_white(amount);
        if !self.era_rules.disable_comeback_scaling
            && mode == GaugeMode::EraDefault
        {
            state.award_v_meter((amount / 4) as u8);
            self.era_share_gauge(side);
        }
        self.era_damage(def_ps, amount, true);
    }
}
