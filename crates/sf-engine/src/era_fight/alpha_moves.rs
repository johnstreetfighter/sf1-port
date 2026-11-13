//! Arcade Alpha commands and their native move slots.
use super::*;
use crate::alpha::Stance;
use crate::moves::MoveInput;
use ButtonClass::*;
use Dir::*;

pub(super) const QCF: &[Dir] = &[Down, DownForward, Forward];
const QCB: &[Dir] = &[Down, DownBack, Back];
const DP: &[Dir] = &[Forward, Down, DownForward];
const RDP: &[Dir] = &[Back, Down, DownBack];
const HCB: &[Dir] = &[Forward, DownForward, Down, DownBack, Back];
const BACK_FORWARD: &[Dir] = &[ChargeBack, Forward];
const DOWN_UP: &[Dir] = &[ChargeDown, Up];
const AIR_QCB_QCB: &[Dir] = &[Jump, Down, DownBack, Back, Down, DownBack, Back];
const BIRDIE_SUPER: &[Dir] = &[ChargeBack, Forward, Back, Forward];
pub(super) const SPIN: &[Dir] = &[Forward, Down, Back, Up];
const ALPHA_2_ON: &[Revision] = &[Revision::Sfz2, Revision::Sfz2A, Revision::Sfz3];

#[derive(Clone, Copy)]
pub(crate) struct AlphaMove {
    pub(crate) slot: u8,
    pub(crate) name: &'static str,
    pub(crate) call_out: &'static str,
    pub(crate) input: MoveInput,
    pub(crate) kind: MoveKind,
    revisions: &'static [Revision],
    x_ism: bool,
    fixed_level: u8,
    pub(crate) airborne: bool,
    pub(crate) overhead: bool,
    x_only: bool,
    stance: Option<Stance>,
    pub(crate) switch_to: Option<Stance>,
    listed: bool,
}

impl AlphaMove {
    pub(super) const fn special(
        slot: u8,
        name: &'static str,
        motion: &'static [Dir],
        button: ButtonClass,
    ) -> Self {
        Self {
            slot,
            name,
            call_out: name,
            input: MoveInput { motion, button },
            kind: MoveKind::Special,
            revisions: ALPHA_REVISIONS,
            x_ism: true,
            fixed_level: 0,
            airborne: false,
            overhead: false,
            x_only: false,
            stance: None,
            switch_to: None,
            listed: true,
        }
    }

    pub(super) const fn overhead_normal(slot: u8, name: &'static str, button: ButtonClass) -> Self {
        Self {
            kind: MoveKind::Normal,
            overhead: true,
            ..Self::special(slot, name, &[Forward], button).revision(ALPHA_2_ON)
        }
    }

    pub(super) const fn super_combo(
        slot: u8,
        name: &'static str,
        call_out: &'static str,
        motion: &'static [Dir],
        button: ButtonClass,
    ) -> Self {
        Self {
            kind: MoveKind::Super,
            x_ism: slot == SF2_SUPER,
            call_out,
            ..Self::special(slot, name, motion, button)
        }
    }

    pub(super) const fn revision(self, revisions: &'static [Revision]) -> Self {
        Self { revisions, ..self }
    }

    pub(super) const fn no_x(self) -> Self {
        Self {
            x_ism: false,
            ..self
        }
    }

    const fn only_x(self) -> Self {
        Self {
            x_only: true,
            ..self
        }
    }

    const fn air(self) -> Self {
        Self {
            airborne: true,
            ..self
        }
    }

    const fn overhead(self) -> Self {
        Self {
            overhead: true,
            ..self
        }
    }

    pub(super) const fn level_three(self) -> Self {
        Self {
            fixed_level: 3,
            ..self
        }
    }

    const fn unlisted(self) -> Self {
        Self {
            listed: false,
            ..self
        }
    }

    const fn in_stance(self, stance: Stance) -> Self {
        Self {
            stance: Some(stance),
            ..self
        }
    }

    const fn switch(name: &'static str, button: ButtonClass, stance: Stance, to: Stance) -> Self {
        Self {
            kind: MoveKind::System,
            switch_to: Some(to),
            ..Self::special(0x1f, name, &[], button)
                .in_stance(stance)
                .no_x()
        }
    }

    pub(super) fn motion(self) -> &'static [Dir] {
        self.input
            .motion
            .strip_prefix(&[Jump])
            .unwrap_or(self.input.motion)
    }

    pub(super) fn available(self, state: &EraFighterState, listed: bool) -> bool {
        self.revisions.contains(&state.style.revision)
            && (if state.style.loadout == Loadout::XIsm {
                self.x_ism
            } else {
                !self.x_only
            })
            && (self.kind != MoveKind::Super
                || (state.super_capacity != 0 && state.style.loadout != Loadout::VIsm))
            && self
                .stance
                .is_none_or(|stance| stance == state.gen_stance())
            && (!listed || self.listed)
    }

    pub(super) fn level(self, state: &EraFighterState, requested: u8) -> Option<u8> {
        if self.kind != MoveKind::Super {
            return Some(0);
        }
        let level = if state.is_alpha_x_ism() {
            3
        } else if self.fixed_level != 0 {
            self.fixed_level
        } else {
            requested
        };
        (state.super_meter >= state.meter_cost(state.alpha_super_cost(level)?)).then_some(level)
    }
}

pub(super) const SAGAT: &[AlphaMove] = &[
    // Start pauses the port, so Angry Charge uses all three punches.
    AlphaMove {
        fixed_level: 1,
        ..AlphaMove::super_combo(0x2f, "ANGRY CHARGE", "ANGRY CHARGE!", QCF, TriplePunch)
            .revision(ALPHA_3)
    },
    AlphaMove::super_combo(
        0x29,
        "TIGER GENOCIDE",
        "TIGER GENOCIDE!",
        SHORT_DOUBLE_QUARTER_CIRCLE,
        Kick,
    ),
    AlphaMove::super_combo(
        0x2a,
        "TIGER CANNON",
        "TIGER CANNON!",
        DOUBLE_QUARTER_CIRCLE,
        Punch,
    ),
    AlphaMove::super_combo(
        0x2b,
        "TIGER RAID",
        "TIGER RAID!",
        DOUBLE_QUARTER_CIRCLE_BACK,
        Kick,
    ),
    AlphaMove::special(0x1f, "TIGER BLOW", DP, Punch).no_x(),
    AlphaMove::special(0x1f, "TIGER UPPERCUT", DP, Punch)
        .revision(ALPHA_3)
        .only_x(),
    AlphaMove::special(0x19, "TIGER CRUSH", DP, Kick).no_x(),
    AlphaMove::special(0x19, "TIGER CRUSH", TIGER_KNEE, Kick)
        .revision(ALPHA_3)
        .only_x(),
];

pub(super) const ADON: &[AlphaMove] = &[
    AlphaMove::super_combo(
        0x29,
        "JAGUAR VARIED ASSAULT",
        "JAGUAR VARIED ASSAULT!",
        SHORT_DOUBLE_QUARTER_CIRCLE,
        Punch,
    ),
    AlphaMove::super_combo(
        0x2a,
        "JAGUAR REVOLVER",
        "JAGUAR REVOLVER!",
        DOUBLE_QUARTER_CIRCLE_BACK,
        Kick,
    ),
    AlphaMove::special(0x19, "RISING JAGUAR", DP, Kick),
    AlphaMove::special(0x19, "RISING JAGUAR", TIGER_KNEE, Kick).unlisted(),
    AlphaMove::special(0x1b, "JAGUAR KICK", RDP, Kick).no_x(),
    AlphaMove::special(0x2d, "AIR JAGUAR KICK", &[Jump], Kick)
        .revision(ALPHA_3)
        .only_x()
        .air(),
    AlphaMove::special(0x2c, "JAGUAR TOOTH", HCB, Kick),
];

pub(super) const BIRDIE: &[AlphaMove] = &[
    AlphaMove::super_combo(0x29, "THE BIRDIE", "THE BIRDIE!", BIRDIE_SUPER, Punch),
    AlphaMove::super_combo(
        0x2a,
        "BULL REVENGER",
        "BULL REVENGER!",
        SHORT_DOUBLE_QUARTER_CIRCLE,
        Either,
    ),
    AlphaMove::special(0x1a, "BULL HEAD", BACK_FORWARD, Punch),
    AlphaMove::special(0x1b, "BULL HORN", &[], ReleasePair),
    AlphaMove::special(0x2c, "MURDERER CHAIN", SPIN, Punch),
    AlphaMove::special(0x2d, "BANDIT CHAIN", SPIN, Kick).revision(ALPHA_2_ON),
];

pub(super) const EAGLE: &[AlphaMove] = &[
    AlphaMove::super_combo(
        0x29,
        "MANCHESTER GOLD",
        "MANCHESTER GOLD!",
        DOUBLE_QUARTER_CIRCLE_BACK,
        Punch,
    ),
    AlphaMove::super_combo(
        0x2a,
        "UNION JACK PLATINUM",
        "UNION JACK PLATINUM!",
        DOUBLE_QUARTER_CIRCLE,
        Punch,
    ),
    AlphaMove::special(0x1a, "MANCHESTER BLACK", QCB, Punch),
    AlphaMove::special(0x2c, "CANTERBURY BLUE", QCF, Punch).no_x(),
    AlphaMove::special(0x2d, "OXFORD RED", HALF_CIRCLE, Kick),
    AlphaMove::special(0x2f, "LIVERPOOL WHITE", QCB, Kick).overhead(),
    AlphaMove::special(0x1f, "ST ANDREWS GREEN", DP, Punch).no_x(),
];

use Stance::{Crane, Mantis};

pub(super) const GEN: &[AlphaMove] = &[
    AlphaMove::super_combo(0x29, "ZAN'EI", "ZAN'EI!", DOUBLE_QUARTER_CIRCLE, Punch)
        .in_stance(Mantis),
    AlphaMove::super_combo(
        0x2a,
        "SHITENSHUU",
        "SHITENSHUU!",
        DOUBLE_QUARTER_CIRCLE_BACK,
        Punch,
    )
    .in_stance(Mantis),
    AlphaMove::super_combo(
        0x2b,
        "JAKOUHA",
        "JAKOUHA!",
        SHORT_DOUBLE_QUARTER_CIRCLE,
        Kick,
    )
    .in_stance(Crane),
    AlphaMove::super_combo(0x2c, "KOUGA", "KOUGA!", AIR_QCB_QCB, Kick)
        .in_stance(Crane)
        .air()
        .overhead(),
    AlphaMove::special(0x1b, "GEKIROU", DP, Kick).in_stance(Mantis),
    AlphaMove::special(0x1a, "HYAKU RENKOU", &[], MashPunch).in_stance(Mantis),
    AlphaMove::special(0x2d, "JASEN", BACK_FORWARD, Punch)
        .in_stance(Crane)
        .no_x(),
    AlphaMove::special(0x2f, "OUGA", DOWN_UP, Kick)
        .in_stance(Crane)
        .no_x()
        .overhead(),
    AlphaMove::switch("SWITCH TO CRANE", TripleKick, Mantis, Crane),
    AlphaMove::switch("SWITCH TO MANTIS", TriplePunch, Crane, Mantis),
];
