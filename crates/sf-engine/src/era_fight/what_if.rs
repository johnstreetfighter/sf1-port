//! What-If commands use each era's native fighter and gauge rules.
use super::{EraFighterState, MotionTracker, SpecialContext, KICK_MASK, PUNCH_MASK, STOCK_AT};
use crate::config::RulesetPreset;
use crate::era::{EraFamily, Loadout, Revision};
use crate::fighter::Bus;
use crate::moves::{Availability, ButtonClass, Dir, MoveEntry, MoveInput, MoveKind, Recognizer};
use crate::roster::TournamentFighter;
use sf_video::compositor::PaletteTint;

use ButtonClass::*;
use Dir::*;
use TournamentFighter::{Geki, Joe, Lee, Mike, Retsu};

const QCF: &[Dir] = &[Down, DownForward, Forward];
const QCB: &[Dir] = &[Down, DownBack, Back];
const DP: &[Dir] = &[Forward, Down, DownForward];
const RDP: &[Dir] = &[Back, Down, DownBack];
const HCF: &[Dir] = &[Back, DownBack, Down, DownForward, Forward];
const HCB: &[Dir] = &[Forward, DownForward, Down, DownBack, Back];
const CHARGE_BF: &[Dir] = &[ChargeBack, Forward];
const AIR_QCF: &[Dir] = &[Jump, Down, DownForward, Forward];
const AIR_QCB: &[Dir] = &[Jump, Down, DownBack, Back];
const QCF2: &[Dir] = super::DOUBLE_QUARTER_CIRCLE;
const QCB2: &[Dir] = super::DOUBLE_QUARTER_CIRCLE_BACK;
const MAX_SPECIALS: usize = 12;
const FINISHER_SLOTS: [u8; 3] = [0x29, 0x2a, 0x2b];

macro_rules! enhanced_names {
    ($name:literal) => { (concat!("EX ", $name), concat!("OD ", $name)) };
}
pub(super) use enhanced_names;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct WhatIfState {
    specials: [MotionTracker; MAX_SPECIALS],
    supers: [MotionTracker; 3],
    punch_taps: u8,
    punch_timer: u8,
    pair_held: u8,
    pair_released: bool,
    ex_slot: u8,
    ex_flash: u8,
    trail: u8,
    pub(super) install_frames: u8,
}

crate::canonical::canonical_struct!(WhatIfState, "WhatIfState.v1", {
    specials, supers, punch_taps, punch_timer, pair_held, pair_released, ex_slot,
    ex_flash, trail, install_frames,
});

#[derive(Clone, Copy)]
pub(crate) struct WhatIfMove {
    slot: u8,
    name: &'static str,
    input: MoveInput,
    enhancement: Option<Enhancement>,
    air: bool,
    overhead: bool,
    crane: Option<bool>,
}

#[derive(Clone, Copy)]
struct Enhancement {
    slot: u8,
    ex_name: &'static str,
    od_name: &'static str,
}

impl WhatIfMove {
    pub(crate) const fn new(
        slot: u8,
        name: &'static str,
        motion: &'static [Dir],
        button: ButtonClass,
    ) -> Self {
        Self {
            slot,
            name,
            input: MoveInput { motion, button },
            enhancement: None,
            air: false,
            overhead: false,
            crane: None,
        }
    }

    pub(crate) const fn enhanced(self, names: (&'static str, &'static str)) -> Self {
        self.enhanced_at(self.slot, names)
    }

    pub(crate) const fn enhanced_at(mut self, slot: u8, (ex_name, od_name): (&'static str, &'static str)) -> Self {
        self.enhancement = Some(Enhancement { slot, ex_name, od_name });
        self
    }

    pub(crate) const fn air(mut self) -> Self {
        self.air = true;
        self
    }

    pub(crate) const fn overhead(mut self) -> Self {
        self.overhead = true;
        self
    }

    pub(crate) const fn stance(mut self, crane: bool) -> Self {
        self.crane = Some(crane);
        self
    }

    fn motion(self) -> &'static [Dir] {
        self.input
            .motion
            .strip_prefix(&[Jump])
            .unwrap_or(self.input.motion)
    }

    fn ex_button(self) -> ButtonClass {
        if self.input.button.kick() {
            PairKick
        } else {
            PairPunch
        }
    }

    fn enhanced_input(self) -> MoveInput {
        MoveInput { button: self.ex_button(), ..self.input }
    }

    fn matches_slot(self, slot: u8) -> bool {
        self.slot == slot || self.enhancement.is_some_and(|enhancement| enhancement.slot == slot)
    }
}

pub(crate) struct WhatIfKit {
    pub(crate) specials: &'static [WhatIfMove],
    pub(crate) supers: [&'static str; 3],
    pub(crate) v_skills: [&'static str; 2],
    pub(crate) v_triggers: [&'static str; 2],
}

const RETSU: WhatIfKit = WhatIfKit {
    specials: &[
        WhatIfMove::new(0x1a, "KIAI SHOT", QCF, Punch).enhanced(enhanced_names!("KIAI SHOT")),
        WhatIfMove::new(0x19, "SHORYUKEN", DP, Punch).enhanced(enhanced_names!("SHORYUKEN")),
        // The half circle also completes Tobi Geri's quarter circle.
        WhatIfMove::new(0x21, "IZUNA SHOULDER THROW", HCF, Kick),
        WhatIfMove::new(0x1b, "TOBI GERI", QCF, Kick).enhanced(enhanced_names!("TOBI GERI")),
        WhatIfMove::new(0x1f, "SENPUU KYAKU", QCB, Kick).enhanced(enhanced_names!("SENPUU KYAKU")),
        WhatIfMove::new(0x20, "HUNDRED MONK SLAPS", &[], MashPunch),
    ],
    supers: ["SHIN KIAIHOU", "KIAI BAKUSAI", "TENSEI RENGE"],
    v_skills: ["SHORINJI PALM THRUST", "RISING DRAGON KNEE"],
    v_triggers: ["KIAI RENBU", "RENKIKO"],
};
const GEKI: WhatIfKit = WhatIfKit {
    specials: &[
        WhatIfMove::new(0x1a, "SHURIKEN", QCF, Punch).enhanced_at(0x24, enhanced_names!("SHURIKEN FAN")),
        WhatIfMove::new(0x1b, "UTSUSEMI", QCB, Punch),
        WhatIfMove::new(0x19, "GEKI-ZAN", DP, Punch).enhanced(enhanced_names!("GEKI-ZAN")),
        WhatIfMove::new(0x1f, "AIR SLASH", AIR_QCF, Kick)
            .air()
            .overhead()
            .enhanced(enhanced_names!("AIR SLASH")),
        WhatIfMove::new(0x20, "EXECUTION DROP", HCB, Kick),
        WhatIfMove::new(0x21, "AIR UTSUSEMI", AIR_QCB, Punch).air(),
        WhatIfMove::new(0x22, "UTSUSEMI FEINT", QCB, Kick),
    ],
    supers: ["MUGEN SHURIKEN", "BUSHIN MUSOU RENGE", "YAMI SHIGURE"],
    v_skills: ["UTSUSEMI FEINT", "CLAW DIVE"],
    v_triggers: ["NINPOU KAGEROU", "SUBSTITUTION TELEPORT"],
};
const JOE: WhatIfKit = WhatIfKit {
    specials: &[
        WhatIfMove::new(0x1a, "SHOCKWAVE", QCF, Kick).enhanced(enhanced_names!("SHOCKWAVE")),
        WhatIfMove::new(0x1b, "ROLLING SOBAT", QCB, Kick).enhanced(enhanced_names!("ROLLING SOBAT")),
        WhatIfMove::new(0x19, "CORKSCREW CROSS", DP, Punch).enhanced(enhanced_names!("CORKSCREW CROSS")),
        WhatIfMove::new(0x1f, "KNEE BASH", DP, Kick).enhanced(enhanced_names!("KNEE BASH")),
        WhatIfMove::new(0x20, "STEP HIGH KICK", &[Forward], Kick),
        WhatIfMove::new(0x21, "FIST COMBINATION", QCF, Punch),
        WhatIfMove::new(0x22, "DUCKING RUSH", QCB, Punch).enhanced(enhanced_names!("DUCKING RUSH")),
    ],
    supers: ["SUPER SOBAT RUSH", "DOUBLE CYCLONE", "FINAL DYNAMITE"],
    v_skills: ["DUCKING RUSH", "KNEE BASH"],
    v_triggers: ["FIST OF LEGEND", "AMERICAN DREAM"],
};
const MIKE: WhatIfKit = WhatIfKit {
    specials: &[
        WhatIfMove::new(0x1a, "MURDER STRAIGHT", QCF, Punch).enhanced(enhanced_names!("MURDER STRAIGHT")),
        WhatIfMove::new(0x19, "WEAVING UPPER", DP, Punch).enhanced(enhanced_names!("WEAVING UPPER")),
        WhatIfMove::new(0x1b, "ROLLING BLOW", QCB, Punch).enhanced(enhanced_names!("ROLLING BLOW")),
        WhatIfMove::new(0x1f, "DUCKING SWAY", QCB, Kick),
    ],
    supers: ["CRAZY RUSH", "LETHAL KNOCKOUT", "CRAZY RUSH FINAL"],
    v_skills: ["DUCKING SWAY", "IRON UPPER"],
    v_triggers: ["POWER RUSH", "LETHAL KNOCKOUT"],
};
const LEE: WhatIfKit = WhatIfKit {
    specials: &[
        WhatIfMove::new(0x1a, "OPEN CANVAS PUNCH", QCF, Punch).enhanced(enhanced_names!("OPEN CANVAS PUNCH")),
        WhatIfMove::new(0x1f, "OPEN CANVAS KICK", QCF, Kick).enhanced(enhanced_names!("OPEN CANVAS KICK")),
        WhatIfMove::new(0x19, "UPWARD LANCE", DP, Kick).enhanced(enhanced_names!("UPWARD LANCE")),
        WhatIfMove::new(0x1b, "HURRICANE KICK", QCB, Kick).enhanced(enhanced_names!("HURRICANE KICK")),
        WhatIfMove::new(0x20, "FIREWATER", CHARGE_BF, Punch).enhanced(enhanced_names!("FIREWATER")),
        WhatIfMove::new(0x21, "STAB KICK", RDP, Kick),
    ],
    supers: ["HUNDRED SPEARS", "FIERY BREATH", "ONE MAN ARMY"],
    v_skills: ["SUNRIDER", "FIREWATER"],
    v_triggers: ["FIERY BREATH", "ONE MAN ARMY"],
};

pub(crate) fn kit(
    fighter: TournamentFighter,
    family: EraFamily,
    crane: bool,
) -> Option<&'static WhatIfKit> {
    match fighter {
        Retsu => Some(&RETSU),
        Geki => Some(&GEKI),
        Joe => Some(&JOE),
        Mike => Some(&MIKE),
        Lee => Some(&LEE),
        _ => super::what_if_others::kit(fighter, family, crane),
    }
}

#[derive(Clone, Copy)]
enum Spend {
    None,
    Super(u8),
    Revenge,
    Drive,
}

#[derive(Clone, Copy)]
struct Finisher {
    slot: u8,
    name: &'static str,
    input: MoveInput,
    spend: Spend,
    level: u8,
}

impl EraFighterState {
    fn what_if_kit(&self) -> Option<&'static WhatIfKit> {
        self.what_if_active()
            .then(|| kit(self.fighter?, self.style.family, self.gen_crane))
            .flatten()
    }

    fn what_if_ex_spend(&self) -> Option<Spend> {
        if self.super_capacity == 0 {
            return None;
        }
        match self.style.family {
            EraFamily::Sf3 if !self.unified_gauge => Some(Spend::Super(40)),
            EraFamily::Sf3 | EraFamily::Sf4 | EraFamily::Sf5 => Some(Spend::Super(STOCK_AT)),
            EraFamily::Sf6 => Some(Spend::Drive),
            _ => None,
        }
    }

    fn what_if_finisher(&self, kit: &WhatIfKit, index: usize, strength: u8) -> Option<Finisher> {
        if self.super_capacity == 0 {
            return None;
        }
        let slot = FINISHER_SLOTS[index];
        let spend = match (self.style.family, index, self.style.loadout) {
            (EraFamily::Sf2, 0, Loadout::Standard) if self.style.revision == Revision::St => {
                Spend::Super(self.super_capacity)
            }
            (EraFamily::SfZ, 0, Loadout::XIsm) => Spend::Super(self.super_capacity),
            (EraFamily::SfZ, _, Loadout::AIsm | Loadout::Standard) => {
                Spend::Super(strength * STOCK_AT)
            }
            (EraFamily::Sf3, 0, Loadout::Art1)
            | (EraFamily::Sf3, 1, Loadout::Art2)
            | (EraFamily::Sf3, 2, Loadout::Art3) => Spend::Super(self.sf3_super_art_cost()?),
            (EraFamily::Sf4, 0, _) => Spend::Super(self.super_capacity),
            (EraFamily::Sf4, 1, Loadout::Ultra1 | Loadout::UltraDouble)
            | (EraFamily::Sf4, 2, Loadout::Ultra2 | Loadout::UltraDouble) => {
                if self.unified_gauge {
                    Spend::Super(3 * STOCK_AT)
                } else {
                    Spend::Revenge
                }
            }
            (EraFamily::Sf5, 0, _) => Spend::Super(3 * STOCK_AT),
            (EraFamily::Sf6, _, _) => Spend::Super((index as u8 + 1) * STOCK_AT),
            _ => return None,
        };
        let input = super::what_if_others::super_input(self.fighter?, self.style.family, slot)
            .unwrap_or_else(|| {
                let (motion, button) = match (self.style.family, index) {
                    (EraFamily::Sf4, 1) => (QCF2, TriplePunch),
                    (EraFamily::Sf4, 2) => (QCF2, TripleKick),
                    (EraFamily::Sf3, _) => (QCF2, Punch),
                    (_, 1) => (QCF2, Kick),
                    (_, 2) => (QCB2, Punch),
                    _ => (QCF2, Punch),
                };
                MoveInput { motion, button }
            });
        let level = if self.style.family == EraFamily::SfZ && self.style.loadout != Loadout::XIsm {
            strength
        } else {
            3
        };
        Some(Finisher {
            slot,
            name: kit.supers[index],
            input,
            spend,
            level,
        })
    }

    pub(crate) fn what_if_moves(&self) -> Vec<MoveEntry> {
        use super::what_if_others::{EAGLE_COUNTER, EAGLE_RIPOSTE};

        let Some(kit) = self.what_if_kit() else {
            return Vec::new();
        };
        let mut entries = Vec::new();
        let special_entry = |slot, name, input, kind| {
            let mut item = entry(slot, name, input, kind);
            if self.fighter == Some(TournamentFighter::Eagle) && slot == EAGLE_COUNTER {
                item.produces = EAGLE_RIPOSTE;
                item.available.fighters = &[TournamentFighter::Eagle];
            }
            item
        };
        for special in kit.specials {
            let kind = if special.crane.is_some() {
                MoveKind::System
            } else {
                MoveKind::Special
            };
            entries.push(special_entry(special.slot, special.name, special.input, kind));
            if let Some(enhancement) = special.enhancement.filter(|_| self.what_if_ex_spend().is_some()) {
                let name = if self.style.family == EraFamily::Sf6 {
                    enhancement.od_name
                } else {
                    enhancement.ex_name
                };
                entries.push(special_entry(
                    enhancement.slot,
                    name,
                    special.enhanced_input(),
                    MoveKind::Special,
                ));
            }
        }
        for index in 0..3 {
            if let Some(finisher) = self.what_if_finisher(kit, index, 1) {
                entries.push(entry(
                    finisher.slot,
                    finisher.name,
                    finisher.input,
                    MoveKind::Super,
                ));
            }
        }
        entries
    }

    pub(crate) fn what_if_progress(&self, slot: u8) -> (u8, u8) {
        let Some(kit) = self.what_if_kit() else {
            return (0, 0);
        };
        if let Some(index) = FINISHER_SLOTS.into_iter().position(|id| id == slot) {
            let tracker = self.what_if_kit.supers[index];
            return (tracker.position, tracker.remaining);
        }
        let command_slot = self.what_if_command_slot(slot);
        kit.specials
            .iter()
            .enumerate()
            .find(|(_, special)| special.matches_slot(command_slot))
            .map_or((0, 0), |(index, special)| {
                let tracker = self.what_if_kit.specials[index];
                let jump = u8::from(special.air && tracker.position != 0);
                (tracker.position + jump, tracker.remaining)
            })
    }

    fn what_if_command_slot(&self, slot: u8) -> u8 {
        use super::what_if_others::{EAGLE_COUNTER, EAGLE_RIPOSTE};

        if self.fighter == Some(TournamentFighter::Eagle) && slot == EAGLE_RIPOSTE {
            EAGLE_COUNTER
        } else {
            slot
        }
    }

    pub(crate) fn what_if_variant_matches(&self, entry: &MoveEntry) -> bool {
        if entry.recognizer != Recognizer::FamilyMove || entry.kind != MoveKind::Special {
            return true;
        }
        let Some(kit) = self.what_if_kit() else {
            return true;
        };
        let slot = self.what_if_command_slot(entry.produces);
        let shared = kit.specials.iter().find(|special| {
            special.slot == slot
                && special.enhancement.is_some_and(|enhancement| enhancement.slot == slot)
                && (entry.input == special.input || entry.input == special.enhanced_input())
        });
        // The native recognizer records the actual enhancement, including
        // whether it could spend meter and whether a counter later landed.
        shared.is_none_or(|special| {
            (entry.input == special.enhanced_input()) == (self.what_if_kit.ex_slot == slot)
        })
    }

    pub(crate) fn what_if_airborne(&self, slot: u8) -> bool {
        self.alpha_move_active(slot)
            && self.what_if_kit().is_some_and(|kit| {
                kit.specials
                    .iter()
                    .any(|special| special.air && special.matches_slot(slot))
            })
    }

    pub(crate) fn what_if_overhead(&self, slot: u8) -> bool {
        self.alpha_move_active(slot)
            && self.what_if_kit().is_some_and(|kit| {
                kit.specials
                    .iter()
                    .any(|special| special.overhead && special.matches_slot(slot))
            })
    }

    pub(crate) fn what_if_special_active(&self, slot: u8) -> bool {
        self.alpha_move_active(slot)
            && self.what_if_kit().is_some_and(|kit| {
                kit.specials
                    .iter()
                    .any(|special| special.crane.is_none() && special.matches_slot(slot))
            })
    }

    pub(crate) fn what_if_projectile_hits(&self, slot: u8) -> Option<u8> {
        if !self.what_if_active() || !self.alpha_move_active(slot) {
            return None;
        }
        match (self.fighter, slot) {
            (Some(Geki), 0x24 | 0x29) => Some(1),
            (Some(Joe | Retsu), 0x1a) if self.what_if_kit.ex_slot == slot => Some(2),
            (Some(TournamentFighter::Sagat), 0x1a | 0x1b) if self.what_if_kit.ex_slot == slot => Some(2),
            _ => None,
        }
    }

    pub(crate) fn what_if_tint(&self) -> Option<PaletteTint> {
        (self.what_if_active()
            && self.what_if_kit.ex_flash != 0
            && self.alpha_move_active(self.what_if_kit.ex_slot))
        .then_some(PaletteTint {
            color: if self.style.family == EraFamily::Sf6 {
                0xff83f147
            } else {
                0xffffcc44
            },
            strength: if self.what_if_kit.ex_flash & 3 == 0 {
                108
            } else {
                64
            },
        })
    }

    pub(crate) fn what_if_trail(&self) -> Option<u32> {
        let color = if self.fighter == Some(TournamentFighter::Eagle) && self.alpha_move_active(0x2b) {
            0xffffc641
        } else {
            0xff86dfff
        };
        (self.what_if_active()
            && (self.what_if_kit.trail != 0 || self.what_if_kit.install_frames != 0))
            .then_some(color)
    }

    pub(crate) fn what_if_tick(&mut self) {
        self.what_if_kit.ex_flash = self.what_if_kit.ex_flash.saturating_sub(1);
        self.what_if_kit.trail = self.what_if_kit.trail.saturating_sub(1);
        self.what_if_kit.install_frames = self.what_if_kit.install_frames.saturating_sub(1);
    }
}

fn entry(slot: u8, name: &'static str, input: MoveInput, kind: MoveKind) -> MoveEntry {
    MoveEntry {
        name,
        kind,
        input,
        recognizer: Recognizer::FamilyMove,
        produces: slot,
        call_out: name,
        available: Availability {
            fighters: &TournamentFighter::ALL,
            presets: &RulesetPreset::ALL,
            player_table: true,
        },
    }
}

fn button_fired(button: ButtonClass, held: u8, fresh: u8) -> bool {
    match button {
        Punch => fresh & PUNCH_MASK != 0,
        Kick => fresh & KICK_MASK != 0,
        _ => button.held(held) && !button.held(held & !fresh),
    }
}

fn button_strength(button: ButtonClass, held: u8) -> u8 {
    let held = if button.kick() {
        held >> 3
    } else {
        held & PUNCH_MASK
    };
    if held & 4 != 0 {
        3
    } else if held & 2 != 0 {
        2
    } else {
        1
    }
}

impl Bus<'_> {
    pub(crate) fn what_if_track(&mut self, ps: u32, relative: u8) {
        let Some(side) = self.era_side(ps) else {
            return;
        };
        let (held, fresh) = self.era_buttons(ps);
        let move_id = self.eb(self.player_entities(ps).0, 0xa);
        let state = &mut self.toggle_state.era.fighters[side];
        let Some(kit) = state.what_if_kit() else {
            return;
        };
        for (index, special) in kit.specials.iter().enumerate() {
            state.what_if_kit.specials[index].step(relative, special.motion());
        }
        for index in 0..3 {
            if let Some(finisher) = state.what_if_finisher(kit, index, 1) {
                state.what_if_kit.supers[index].step(relative, finisher.input.motion);
            }
        }
        let runtime = &mut state.what_if_kit;
        runtime.punch_timer = runtime.punch_timer.saturating_sub(1);
        if runtime.punch_timer == 0 {
            runtime.punch_taps = 0;
        }
        if fresh & PUNCH_MASK != 0 {
            runtime.punch_taps = runtime.punch_taps.saturating_add(1);
            runtime.punch_timer = 24;
        }
        let pair = ReleasePair.held(held);
        runtime.pair_released = !pair && runtime.pair_held >= super::CHARGE_FRAMES as u8;
        runtime.pair_held = if pair {
            runtime.pair_held.saturating_add(1)
        } else {
            0
        };
        if state.fighter == Some(TournamentFighter::Gen)
            && move_id == 0x1b
            && fresh & KICK_MASK != 0
        {
            state.repeated_kick = true;
        }
    }

    fn what_if_can_spend(&self, ps: u32, state: &EraFighterState, spend: Spend) -> bool {
        match spend {
            Spend::None => true,
            Spend::Super(cost) => state.super_meter >= state.meter_cost(cost),
            Spend::Revenge => state.is_sf4() && state.sf4_ultra_ready(),
            Spend::Drive => self.sf6_can_spend(ps, 96),
        }
    }

    pub(crate) fn try_what_if_move(
        &mut self,
        ps: u32,
        entity: u16,
        context: SpecialContext,
    ) -> bool {
        let Some(side) = self.era_side(ps) else {
            return false;
        };
        let (held, fresh) = self.era_buttons(ps);
        let state = &self.toggle_state.era.fighters[side];
        let Some(kit) = state.what_if_kit() else {
            return false;
        };
        let airborne = context == SpecialContext::Airborne;
        let canceling = matches!(context, SpecialContext::Ground { canceling: true });
        let finisher = matches!(context, SpecialContext::Ground { .. })
            .then(|| {
                [1, 2, 0].into_iter().find_map(|index| {
                    let initial = state.what_if_finisher(kit, index, 1)?;
                    let strength = button_strength(initial.input.button, held);
                    let finisher = state.what_if_finisher(kit, index, strength)?;
                    let plain_sf4_super = state.style.family == EraFamily::Sf4
                        && index == 0
                        && TriplePunch.held(held);
                    if plain_sf4_super
                        || !button_fired(finisher.input.button, held, fresh)
                        || !state.what_if_kit.supers[index].ready(finisher.input.motion)
                        || !self.what_if_can_spend(ps, state, finisher.spend)
                    {
                        return None;
                    }
                    Some(finisher)
                })
            })
            .flatten();
        if let Some(finisher) = finisher {
            if !self.era_start_banked_move(
                ps,
                entity,
                finisher.slot,
                finisher.level,
                Some(finisher.name),
            ) {
                return false;
            }
            let state = &mut self.toggle_state.era.fighters[side];
            match finisher.spend {
                Spend::Super(cost) => state.super_meter -= state.meter_cost(cost),
                _ => {}
            }
            if state.style.family == EraFamily::Sf4 && matches!(finisher.slot, 0x2a | 0x2b) {
                state.sf4_spend_ultra(self.era_rules.disable_comeback_scaling);
            }
            let runtime = &mut state.what_if_kit;
            runtime.trail = 54;
            runtime.supers = [MotionTracker::default(); 3];
            runtime.specials = [MotionTracker::default(); MAX_SPECIALS];
            if state.fighter == Some(TournamentFighter::Gen)
                && state.style.family == EraFamily::Sf3
                && state.style.loadout == Loadout::Art3
            {
                runtime.install_frames = 120;
            }
            self.era_share_gauge(side);
            return true;
        }

        let intent = kit
            .specials
            .iter()
            .enumerate()
            .find_map(|(index, &special)| {
                if special.air != airborne
                    || canceling && special.crane.is_some()
                    || context == SpecialContext::MashCancel && special.input.button != MashPunch
                {
                    return None;
                }
                if !state.what_if_kit.specials[index].ready(special.motion()) {
                    return None;
                }
                if let Some((slot, spend)) = special.enhancement.and_then(|enhancement| {
                    let spend = state.what_if_ex_spend()?;
                    (button_fired(special.ex_button(), held, fresh)
                        && self.what_if_can_spend(ps, state, spend))
                    .then_some((enhancement.slot, spend))
                }) {
                    return Some((special, slot, spend, true));
                }
                let fired = match special.input.button {
                    MashPunch => fresh & PUNCH_MASK != 0 && state.what_if_kit.punch_taps >= 4,
                    ReleasePair => state.what_if_kit.pair_released,
                    button => button_fired(button, held, fresh),
                };
                fired.then_some((special, special.slot, Spend::None, false))
            });
        let Some((special, slot, spend, ex)) = intent else {
            return false;
        };
        let air_y = airborne.then(|| self.center_y(entity));
        let level = u8::from(ex && special.slot == slot);
        if !self.era_start_banked_move(ps, entity, slot, level, None) {
            return false;
        }
        if let Some(y) = air_y {
            self.era_adjust_air_start(ps, entity, y);
        }
        if matches!(spend, Spend::Drive) {
            self.sf6_spend(ps, 96);
        }
        let state = &mut self.toggle_state.era.fighters[side];
        if let Spend::Super(cost) = spend {
            state.super_meter -= state.meter_cost(cost);
        }
        if let Some(crane) = special.crane {
            state.gen_crane = crane;
        }
        state.repeated_kick = false;
        let runtime = &mut state.what_if_kit;
        runtime.ex_slot = if ex { slot } else { 0 };
        runtime.ex_flash = if ex { 20 } else { 0 };
        runtime.specials = [MotionTracker::default(); MAX_SPECIALS];
        runtime.supers = [MotionTracker::default(); 3];
        runtime.punch_taps = 0;
        runtime.pair_released = false;
        let strength = if ex {
            3
        } else {
            button_strength(special.input.button, held)
        };
        self.set_eb(entity, 0x34, strength);
        self.era_share_gauge(side);
        true
    }
}
