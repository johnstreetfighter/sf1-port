//! Native Street Fighter III commands, Super Art meter, and contact windows.

use super::*;
use crate::config::RulesetPreset;
use crate::moves::{Availability, MoveEntry, MoveInput, Recognizer};
use sf_video::compositor::PaletteTint;

pub(crate) const JOUDAN: u8 = 0x1f;
pub(crate) const AIR_TATSU: u8 = 0x20;
pub(crate) const EX_AIR_TATSU: u8 = 0x21;
pub(crate) const EX_HADOKEN: u8 = 0x24;
pub(crate) const EX_SHORYUKEN: u8 = 0x25;
pub(crate) const EX_TATSUMAKI: u8 = 0x26;
pub(crate) const ART_1: u8 = 0x29;
pub(crate) const ART_2: u8 = 0x2a;
pub(crate) const ART_3: u8 = 0x2b;
pub(crate) const EX_JOUDAN: u8 = 0x2c;
pub(crate) const PARRY: u8 = 0x2e;
pub(crate) const OVERHEAD: u8 = 0x2f;

const PARRY_WINDOW: u8 = 6;
const PARRY_PROJECTILE_WINDOW: u8 = 3;
const RED_WINDOW: u8 = 3;
const RED_PROJECTILE_WINDOW: u8 = 2;
const EX_COST: u8 = 40;
const LONGEST_ART_BAR: u8 = 128;

const DP: &[Dir] = &[Dir::Forward, Dir::Down, Dir::DownForward];
const QCF: &[Dir] = &[Dir::Down, Dir::DownForward, Dir::Forward];
const QCB: &[Dir] = &[Dir::Down, Dir::DownBack, Dir::Back];
const AIR_QCB: &[Dir] = &[Dir::Jump, Dir::Down, Dir::DownBack, Dir::Back];

#[derive(Clone, Copy)]
struct Art {
    name: &'static str,
    call_out: &'static str,
    slot: u8,
    button: ButtonClass,
    bar: u8,
    stocks: u8,
    melee_hits: u8,
}

const RYU_ARTS: [Art; 3] = [
    Art {
        name: "SHINKUU HADOKEN",
        call_out: "SHINKUU HADOKEN!",
        slot: ART_1,
        button: ButtonClass::Punch,
        bar: 112,
        stocks: 2,
        melee_hits: 1,
    },
    Art {
        name: "SHIN SHORYUKEN",
        call_out: "SHIN SHORYUKEN!",
        slot: ART_2,
        button: ButtonClass::Punch,
        bar: 128,
        stocks: 1,
        melee_hits: 4,
    },
    Art {
        name: "DENJIN HADOKEN",
        call_out: "DENJIN HADOKEN!",
        slot: ART_3,
        button: ButtonClass::Punch,
        bar: 96,
        stocks: 1,
        melee_hits: 1,
    },
];

const KEN_ARTS: [Art; 3] = [
    Art {
        name: "SHORYUREPPA",
        call_out: "SHORYUREPPA!",
        slot: ART_1,
        button: ButtonClass::Punch,
        bar: 96,
        stocks: 2,
        melee_hits: 3,
    },
    Art {
        name: "SHINRYUKEN",
        call_out: "SHINRYUKEN!",
        slot: ART_2,
        button: ButtonClass::Kick,
        bar: 112,
        stocks: 1,
        melee_hits: 5,
    },
    Art {
        name: "SHIPPU JINRAIKYAKU",
        call_out: "SHIPPU JINRAIKYAKU!",
        slot: ART_3,
        button: ButtonClass::Kick,
        bar: 80,
        stocks: 3,
        melee_hits: 5,
    },
];

#[derive(Clone, Copy)]
struct Command {
    slot: u8,
    name: &'static str,
    motion: &'static [Dir],
    button: ButtonClass,
}

const BLADE_KICK: Command = Command {
    slot: JOUDAN,
    name: "JOUDAN SOKUTOGERI",
    motion: HALF_CIRCLE,
    button: ButtonClass::Kick,
};
const UNIVERSAL_OVERHEAD: Command = Command {
    slot: OVERHEAD,
    name: "UNIVERSAL OVERHEAD",
    motion: &[],
    button: ButtonClass::MediumPair,
};
const STANDING_PARRY: Command = Command {
    slot: PARRY,
    name: "PARRY",
    motion: &[Dir::Forward],
    button: ButtonClass::DirectionOnly,
};
const AIR_HURRICANE: Command = Command {
    slot: AIR_TATSU,
    name: "AIR TATSUMAKI",
    motion: AIR_QCB,
    button: ButtonClass::Kick,
};
const EX_AIR_HURRICANE: Command = Command {
    slot: EX_AIR_TATSU,
    name: "EX AIR TATSUMAKI",
    motion: AIR_QCB,
    button: ButtonClass::PairKick,
};
const EX_COMMANDS: [Command; 4] = [
    Command {
        slot: EX_HADOKEN,
        name: "EX HADOKEN",
        motion: QCF,
        button: ButtonClass::PairPunch,
    },
    Command {
        slot: EX_SHORYUKEN,
        name: "EX SHORYUKEN",
        motion: DP,
        button: ButtonClass::PairPunch,
    },
    Command {
        slot: EX_TATSUMAKI,
        name: "EX TATSUMAKI",
        motion: QCB,
        button: ButtonClass::PairKick,
    },
    Command {
        slot: EX_JOUDAN,
        name: "EX JOUDAN SOKUTOGERI",
        motion: HALF_CIRCLE,
        button: ButtonClass::PairKick,
    },
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Sf3State {
    super_motion: MotionTracker,
    special_motions: [MotionTracker; EX_COMMANDS.len()],
    previous_direction: u8,
    parry_left: u8,
    parry_low: bool,
    parry_airborne: bool,
    red_parry: bool,
    parry_flash: u8,
    red_flash: bool,
    ex_flash: u8,
    wall_launch: u8,
    wall_rebound: u8,
    rebound_velocity: i8,
    unified: bool,
}

crate::canonical::canonical_struct!(Sf3State, "Sf3State.v1", {
    super_motion, special_motions, previous_direction, parry_left, parry_low,
    parry_airborne, red_parry, parry_flash, red_flash, ex_flash, wall_launch, wall_rebound, rebound_velocity, unified,
});

#[derive(Clone, Copy)]
pub(crate) struct Sf3Projectile {
    pub hits: u8,
    pub unblockable: bool,
}

fn move_entry(
    command: Command,
    kind: MoveKind,
    call_out: &'static str,
    fighter: TournamentFighter,
) -> MoveEntry {
    let fighters: &'static [TournamentFighter] = match fighter {
        TournamentFighter::Ryu => &[TournamentFighter::Ryu],
        TournamentFighter::Ken => &[TournamentFighter::Ken],
        _ => &TournamentFighter::ALL,
    };
    MoveEntry {
        name: command.name,
        kind,
        input: MoveInput {
            motion: command.motion,
            button: command.button,
        },
        recognizer: Recognizer::FamilyMove,
        produces: command.slot,
        call_out,
        available: Availability {
            fighters,
            presets: &[
                RulesetPreset::DefinitiveOverhaul,
                RulesetPreset::ClassicCompetitive,
                RulesetPreset::PureFootsies1987Plus,
                RulesetPreset::Custom,
            ],
            player_table: false,
        },
    }
}

fn art_index(loadout: Loadout) -> Option<usize> {
    match loadout {
        Loadout::Art1 => Some(0),
        Loadout::Art2 => Some(1),
        Loadout::Art3 => Some(2),
        _ => None,
    }
}

fn shoto_art(fighter: TournamentFighter, revision: Revision, index: usize) -> Option<Art> {
    let mut art = match fighter {
        TournamentFighter::Ryu => RYU_ARTS[index],
        TournamentFighter::Ken => KEN_ARTS[index],
        _ => return None,
    };
    if revision == Revision::Sf3Ng {
        art.stocks = if fighter == TournamentFighter::Ken && index != 0 {
            2
        } else {
            1
        };
        if fighter == TournamentFighter::Ryu && index == 1 {
            art.melee_hits = 3;
        }
    }
    Some(art)
}

pub(crate) fn style_art(
    fighter: TournamentFighter,
    revision: Revision,
    loadout: Loadout,
) -> Option<(&'static str, u8)> {
    let index = art_index(loadout)?;
    if let Some(art) = shoto_art(fighter, revision, index) {
        return Some((art.name, art.stocks));
    }
    let kit = super::what_if::kit(fighter, EraFamily::Sf3, false)?;
    Some((kit.supers[index], 3))
}

impl EraFighterState {
    fn sf3_art(&self) -> Option<Art> {
        if self.style.family != EraFamily::Sf3 || self.max_sub_hp == 0 {
            return None;
        }
        shoto_art(
            self.fighter?,
            self.style.revision,
            art_index(self.style.loadout)?,
        )
    }

    pub(crate) fn is_sf3(&self) -> bool {
        self.sf3_art().is_some()
            || (self.what_if && self.max_sub_hp != 0 && self.style.family == EraFamily::Sf3)
    }

    pub(crate) fn sf2_air_tatsumaki_available(&self) -> bool {
        self.is_sf2()
            && self.fighter.is_some_and(TournamentFighter::is_shoto)
            && matches!(
                self.style.revision,
                Revision::Hf | Revision::Super | Revision::St
            )
    }

    pub(crate) fn sf2_air_tatsumaki_entry(&self) -> Option<MoveEntry> {
        let fighter = self
            .fighter
            .filter(|_| self.sf2_air_tatsumaki_available())?;
        Some(move_entry(
            AIR_HURRICANE,
            MoveKind::Special,
            "AIR TATSUMAKI!",
            fighter,
        ))
    }

    pub(crate) fn sf3_set_gauge_mode(&mut self, mode: GaugeMode) {
        self.sf3.unified = mode == GaugeMode::Unified;
        if mode == GaugeMode::NoGauges {
            self.sf3.parry_left = 0;
            self.sf3.parry_flash = 0;
            self.sf3.ex_flash = 0;
        }
    }

    pub(crate) fn sf3_stock_cost(&self) -> Option<u8> {
        self.is_sf3().then(|| {
            if self.sf3.unified {
                STOCK_AT
            } else {
                self.sf3_art().map_or(STOCK_AT, |art| art.bar)
            }
        })
    }

    pub(crate) fn sf3_super_art_cost(&self) -> Option<u8> {
        let stock = self.sf3_stock_cost()?;
        if self.sf3.unified {
            Some(stock * (art_index(self.style.loadout)? as u8 + 1))
        } else {
            Some(stock)
        }
    }

    pub(crate) fn sf3_capacity(&self) -> Option<u8> {
        self.is_sf3().then(|| {
            self.sf3_art()
                .map_or(3 * STOCK_AT, |art| art.bar * art.stocks)
        })
    }

    pub(crate) fn sf3_gauge(&self) -> Option<EraGauge> {
        let art = self
            .sf3_art()
            .filter(|_| self.super_capacity != 0 && !self.sf3.unified)?;
        let stocks = self.super_meter / art.bar;
        let fill = if stocks >= art.stocks {
            Fill::FULL
        } else {
            Fill::of((self.super_meter % art.bar).into(), art.bar.into())
        };
        Some(EraGauge::one(
            GaugeRow::level(stocks, art.stocks, fill)
                .with_length(Fill::of(art.bar.into(), LONGEST_ART_BAR.into())),
        ))
    }

    fn sf3_ex_enabled(&self) -> bool {
        self.super_capacity != 0
            && matches!(self.style.revision, Revision::Sf3TwoI | Revision::Sf3ThreeS)
    }

    pub(super) fn sf3_ex_cost(&self) -> u8 {
        if self.sf3.unified { STOCK_AT } else { EX_COST }
    }

    pub(super) fn sf3_has_ex(&self) -> bool {
        self.sf3_ex_enabled() && self.sf3_ex_commands().next().is_some()
    }

    pub(super) fn modern_sf3(&mut self, entry: MoveEntry, motion: MotionTracker) -> bool {
        if self.sf3_art().is_some_and(|art| art.slot == entry.produces) {
            self.sf3.special_motions.fill(MotionTracker::default());
            self.sf3.super_motion = motion;
            return true;
        }
        if entry.produces == JOUDAN {
            self.sf3.super_motion = MotionTracker::default();
            self.sf3.special_motions.fill(MotionTracker::default());
            self.sf3.special_motions[3] = motion;
            return true;
        }
        false
    }

    fn sf3_ex_commands(&self) -> impl Iterator<Item = (usize, Command)> {
        let ryu = self.fighter == Some(TournamentFighter::Ryu);
        EX_COMMANDS
            .into_iter()
            .enumerate()
            .filter(move |(_, command)| command.slot != EX_JOUDAN || ryu)
    }

    pub(crate) fn sf3_active_moves(&self) -> Vec<MoveEntry> {
        let Some(fighter) = self.fighter.filter(|_| self.is_sf3()) else {
            return Vec::new();
        };
        let mut moves = Vec::new();
        if self.super_capacity != 0 {
            moves.push(move_entry(STANDING_PARRY, MoveKind::System, "PARRY!", fighter));
            moves.push(move_entry(
                UNIVERSAL_OVERHEAD,
                MoveKind::System,
                "UNIVERSAL OVERHEAD!",
                fighter,
            ));
        }
        let Some(art) = self.sf3_art() else {
            return moves;
        };
        if self.fighter == Some(TournamentFighter::Ryu) {
            moves.push(move_entry(
                BLADE_KICK,
                MoveKind::Special,
                "JOUDAN SOKUTOGERI!",
                fighter,
            ));
        }
        if self.style.revision != Revision::Sf3Ng {
            moves.push(move_entry(
                AIR_HURRICANE,
                MoveKind::Special,
                "AIR TATSUMAKI!",
                fighter,
            ));
        }
        if self.super_capacity != 0 {
            let command = Command {
                slot: art.slot,
                name: art.name,
                motion: DOUBLE_QUARTER_CIRCLE,
                button: art.button,
            };
            moves.push(move_entry(command, MoveKind::Super, art.call_out, fighter));
        }
        if self.sf3_ex_enabled() {
            moves.push(move_entry(
                EX_AIR_HURRICANE,
                MoveKind::Special,
                "EX AIR TATSUMAKI!",
                fighter,
            ));
            moves.extend(
                self.sf3_ex_commands().map(|(_, command)| {
                    move_entry(command, MoveKind::Special, command.name, fighter)
                }),
            );
        }
        moves
    }

    pub(crate) fn sf3_progress(&self, slot: u8) -> (u8, u8) {
        if slot == PARRY {
            let state = &self.sf3;
            let high = !state.parry_low && !state.parry_airborne
                && (state.parry_left != 0 || state.parry_flash != 0);
            return (u8::from(high), state.parry_left);
        }
        let (tracker, air) = match slot {
            ART_1..=ART_3 => (self.sf3.super_motion, false),
            JOUDAN | EX_JOUDAN => (self.sf3.special_motions[3], false),
            AIR_TATSU | EX_AIR_TATSU => (self.sf3.special_motions[2], true),
            EX_HADOKEN..=EX_TATSUMAKI => (
                self.sf3.special_motions[usize::from(slot - EX_HADOKEN)],
                false,
            ),
            _ => return (0, 0),
        };
        let jump = u8::from(air && tracker.position != 0);
        (tracker.position + jump, tracker.remaining)
    }

    pub(crate) fn sf3_tint(&self) -> Option<PaletteTint> {
        if self.sf3.parry_flash != 0 {
            let color = if self.sf3.red_flash {
                0xffff652d
            } else {
                0xffa9eaff
            };
            Some(PaletteTint {
                color,
                strength: 210,
            })
        } else if self.sf3.ex_flash != 0 {
            Some(PaletteTint {
                color: 0xffffc44f,
                strength: if self.sf3.ex_flash & 2 == 0 { 185 } else { 116 },
            })
        } else {
            None
        }
    }

    pub(crate) fn sf3_tick(&mut self) {
        self.sf3.parry_flash = self.sf3.parry_flash.saturating_sub(1);
        self.sf3.ex_flash = self.sf3.ex_flash.saturating_sub(1);
        self.sf3.wall_launch = self.sf3.wall_launch.saturating_sub(1);
        self.sf3.wall_rebound = self.sf3.wall_rebound.saturating_sub(1);
    }

    pub(crate) fn sf3_hit_limit(&self, slot: u8) -> Option<u8> {
        let art = self.sf3_art().filter(|_| self.alpha_move_active(slot))?;
        if art.slot == slot {
            return Some(art.melee_hits);
        }
        Some(match slot {
            EX_SHORYUKEN | EX_TATSUMAKI | EX_AIR_TATSU => 3,
            _ => 1,
        })
    }

    pub(crate) fn sf3_overhead(&self, slot: u8) -> bool {
        self.is_sf3() && slot == OVERHEAD && self.alpha_move_active(slot)
    }

    fn sf3_wall_bounce(&self, slot: u8) -> bool {
        self.is_sf3()
            && self.fighter == Some(TournamentFighter::Ryu)
            && slot == EX_JOUDAN
            && self.alpha_move_active(slot)
    }

    pub(crate) fn sf3_projectile(&self, slot: u8) -> Option<Sf3Projectile> {
        if self.sf3_art().is_none() || !self.alpha_move_active(slot) {
            return None;
        }
        let (hits, unblockable) = match (self.fighter, slot) {
            (Some(TournamentFighter::Ryu), ART_1) => (5, false),
            (Some(TournamentFighter::Ryu), ART_3) => {
                (2 + (self.held_windup_frames / 12).min(3), true)
            }
            (_, EX_HADOKEN) => (2, false),
            _ => return None,
        };
        Some(Sf3Projectile { hits, unblockable })
    }
}

impl Bus<'_> {
    pub(crate) fn sf3_track_input(
        &mut self,
        ps: u32,
        relative: u8,
        airborne: bool,
        actionable: bool,
        blockstun: bool,
    ) {
        let Some(side) = self.era_side(ps) else {
            return;
        };
        let state = &mut self.toggle_state.era.fighters[side];
        if state.sf2_air_tatsumaki_available() {
            state.sf3.special_motions[2].step(relative, QCB);
            return;
        }
        if !state.is_sf3() {
            return;
        }
        let sf3 = &mut state.sf3;
        sf3.super_motion.step(relative, DOUBLE_QUARTER_CIRCLE);
        for (tracker, command) in sf3.special_motions.iter_mut().zip(EX_COMMANDS) {
            tracker.step(relative, command.motion);
        }

        let red = blockstun && state.style.revision == Revision::Sf3ThreeS;
        let forward = relative == Dir::Forward.bits();
        let low = !airborne && relative == Dir::Down.bits();
        if state.super_capacity != 0
            && (actionable || red)
            && relative != sf3.previous_direction
            && (forward || low)
        {
            sf3.parry_left = if red { RED_WINDOW } else { PARRY_WINDOW };
            sf3.parry_low = low;
            sf3.parry_airborne = airborne;
            sf3.red_parry = red;
        } else {
            sf3.parry_left = sf3.parry_left.saturating_sub(1);
        }
        sf3.previous_direction = relative;
    }

    pub(crate) fn try_sf3_move(&mut self, ps: u32, entity: u16, airborne: bool) -> bool {
        let Some(side) = self.era_side(ps) else {
            return false;
        };
        let (held, fresh) = self.era_buttons(ps);
        let state = &self.toggle_state.era.fighters[side];
        if airborne && state.sf2_air_tatsumaki_available() {
            if fresh & KICK_MASK == 0 || !state.sf3.special_motions[2].ready(QCB) {
                return false;
            }
            let center = self.center_y(entity);
            if !self.era_start_banked_move(ps, entity, AIR_TATSU, 0, None) {
                return false;
            }
            self.era_adjust_air_start(ps, entity, center);
            self.toggle_state.era.fighters[side].sf3.special_motions[2] = MotionTracker::default();
            return true;
        }
        let overhead = !airborne
            && state.super_capacity != 0
            && ButtonClass::MediumPair.held(held)
            && fresh & ButtonClass::MediumPair.fixed_mask() != 0;
        if self.world_tour_pure_footsies() {
            return state.is_sf3() && overhead
                && self.era_start_banked_move(ps, entity, OVERHEAD, 0, None);
        }
        let Some(art) = state.sf3_art() else {
            return state.is_sf3()
                && overhead
                && self.era_start_banked_move(ps, entity, OVERHEAD, 0, None);
        };
        let meter = state.super_meter;
        let ex_cost = state.meter_cost(state.sf3_ex_cost());
        let super_cost = state.meter_cost(state.sf3_super_art_cost().unwrap_or_default());
        let (slot, cost, level, call_out) = if airborne {
            if state.style.revision == Revision::Sf3Ng
                || fresh & KICK_MASK == 0
                || !state.sf3.special_motions[2].ready(QCB)
            {
                return false;
            }
            if state.sf3_ex_enabled() && meter >= ex_cost && ButtonClass::PairKick.held(held) {
                (EX_AIR_TATSU, ex_cost, 0, None)
            } else {
                (AIR_TATSU, 0, 0, None)
            }
        } else if state.super_capacity != 0
            && art.button.held(fresh)
            && state.sf3.super_motion.ready(DOUBLE_QUARTER_CIRCLE)
            && meter >= super_cost
        {
            (art.slot, super_cost, 1, Some(art.name))
        } else if let Some((_, command)) = state.sf3_ex_commands().find(|&(index, command)| {
            state.sf3_ex_enabled()
                && meter >= ex_cost
                && command.button.held(held)
                && fresh
                    & if command.button.kick() {
                        KICK_MASK
                    } else {
                        PUNCH_MASK
                    }
                    != 0
                && state.sf3.special_motions[index].ready(command.motion)
        }) {
            (command.slot, ex_cost, 0, None)
        } else if overhead {
            (OVERHEAD, 0, 0, None)
        } else if state.fighter == Some(TournamentFighter::Ryu)
            && ButtonClass::Kick.held(fresh)
            && state.sf3.special_motions[3].ready(BLADE_KICK.motion)
        {
            (JOUDAN, 0, 0, None)
        } else {
            return false;
        };

        let air_center = airborne.then(|| self.center_y(entity));
        if !self.era_start_banked_move(ps, entity, slot, level, call_out) {
            return false;
        }
        if let Some(center) = air_center {
            self.era_adjust_air_start(ps, entity, center);
        }
        let state = &mut self.toggle_state.era.fighters[side];
        state.super_meter -= cost;
        state.sf3.super_motion = MotionTracker::default();
        state.sf3.special_motions = [MotionTracker::default(); EX_COMMANDS.len()];
        state.sf3.parry_left = 0;
        if matches!(
            slot,
            EX_HADOKEN | EX_SHORYUKEN | EX_TATSUMAKI | EX_JOUDAN | EX_AIR_TATSU
        ) {
            state.sf3.ex_flash = 8;
        }
        self.era_share_gauge(side);
        true
    }

    pub(crate) fn sf3_mark_wall_launch(&mut self, owner: u32, target: u32, slot: u8) {
        let Some(target_side) = self.era_side(target) else {
            return;
        };
        let launch = self
            .era_state(owner)
            .is_some_and(|state| state.sf3_wall_bounce(slot));
        let sf3 = &mut self.toggle_state.era.fighters[target_side].sf3;
        sf3.wall_launch = if launch { 26 } else { 0 };
        sf3.wall_rebound = 0;
    }

    pub(crate) fn sf3_wall_contact(&mut self, entity: u16, right_wall: bool) -> bool {
        if self.eb(entity, 8) != 2 || !(0x30..=0x3f).contains(&self.eb(entity, 0xa)) {
            return false;
        }
        let ps = self.el(entity, 0x38);
        let Some(side) = self.era_side(ps) else {
            return false;
        };
        let sf3 = &mut self.toggle_state.era.fighters[side].sf3;
        if sf3.wall_launch == 0 {
            return false;
        }
        sf3.wall_launch = 0;
        sf3.wall_rebound = 10;
        sf3.rebound_velocity = if right_wall { -8 } else { 8 };
        self.queue_sound(9);
        true
    }

    pub(crate) fn sf3_wall_motion(&self, entity: u16) -> i8 {
        if self.eb(entity, 8) != 2 || !(0x30..=0x3f).contains(&self.eb(entity, 0xa)) {
            return 0;
        }
        self.era_state(self.el(entity, 0x38)).map_or(0, |state| {
            if state.sf3.wall_rebound > 0 {
                state.sf3.rebound_velocity
            } else {
                0
            }
        })
    }

    pub(crate) fn sf3_parry(
        &mut self,
        defender: u16,
        attack_x: u16,
        low: bool,
        projectile: bool,
        airborne: bool,
    ) -> bool {
        let ps = self.el(defender, 0x38);
        let Some(side) = self.era_side(ps) else {
            return false;
        };
        let state = &self.toggle_state.era.fighters[side];
        if !state.is_sf3() || state.super_capacity == 0 {
            return false;
        }
        let sf3 = &state.sf3;
        let red = sf3.red_parry;
        let window = if red { RED_WINDOW } else { PARRY_WINDOW };
        let maximum_age = match (red, projectile) {
            (true, true) => RED_PROJECTILE_WINDOW,
            (true, false) => RED_WINDOW,
            (false, true) => PARRY_PROJECTILE_WINDOW,
            (false, false) => PARRY_WINDOW,
        };
        let facing_left = self.eb(defender, 0x37) & 4 != 0;
        let towards_attack = (attack_x < self.center_x(defender)) == facing_left;
        if sf3.parry_left == 0
            || window - sf3.parry_left >= maximum_age
            || sf3.parry_airborne != airborne
            || (!airborne && sf3.parry_low != low)
            || (!sf3.parry_low && !towards_attack)
        {
            return false;
        }

        if !airborne {
            self.era_start_banked_move(ps, defender, PARRY, 0, None);
        }
        let state = &mut self.toggle_state.era.fighters[side];
        state.sf3.parry_left = 0;
        state.sf3.parry_flash = 2;
        state.sf3.red_flash = red;
        self.era_meter_gain(ps, 8);
        self.toggle_state.era.freeze = PARRY_WINDOW;
        if side == 0 {
            let contacts = &mut self.toggle_state.era.player_parry_contacts;
            *contacts = contacts.saturating_add(1);
        }
        self.queue_sound(8);
        true
    }
}
