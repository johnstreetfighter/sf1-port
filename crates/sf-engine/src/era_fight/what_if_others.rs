//! Post-1987 What-If commands for fighters with existing Alpha streams.

use super::what_if::{enhanced_names, WhatIfKit, WhatIfMove};
use super::{EraFighterState, DOUBLE_QUARTER_CIRCLE, HALF_CIRCLE};
use crate::era::EraFamily;
use crate::moves::{ButtonClass, Dir, MoveInput};
use crate::roster::TournamentFighter;

use ButtonClass::*;
use Dir::*;
use TournamentFighter::{Adon, Birdie, Eagle, Gen, Sagat};

pub(crate) const EAGLE_COUNTER: u8 = 0x1f;
pub(crate) const EAGLE_RIPOSTE: u8 = 0x23;
pub(crate) const NEXUS: u8 = 0x20;
pub(crate) const MIGHTY_TIGER: u8 = 0x21;
pub(crate) const GREEDY_TIGER: u8 = 0x22;
pub(crate) const NOVA_TIGER: u8 = 0x23;
pub(crate) const OD_NEXUS: u8 = 0x24;

const DP: &[Dir] = &[Forward, Down, DownForward];
const RDP: &[Dir] = &[Back, Down, DownBack];
const QCF: &[Dir] = &[Down, DownForward, Forward];
const QCB: &[Dir] = &[Down, DownBack, Back];
const QCB_TWICE: &[Dir] = &[Down, DownBack, Back, Down, DownBack, Back];
const HCB: &[Dir] = &[Forward, DownForward, Down, DownBack, Back];
const CHARGED_FORWARD: &[Dir] = &[ChargeBack, Forward];
const CHARGED_UP: &[Dir] = &[ChargeDown, Up];
const SPIN: &[Dir] = &[Forward, Down, Back, Up];

const GEN_MANTIS: &[WhatIfMove] = &[
    WhatIfMove::new(0x1a, "HYAKU RENKOU", &[], MashPunch),
    WhatIfMove::new(0x1b, "GEKIROU", DP, Kick).enhanced(enhanced_names!("GEKIROU")),
    WhatIfMove::new(0x1f, "SWITCH TO CRANE", &[], TripleKick).stance(true),
];
const GEN_CRANE: &[WhatIfMove] = &[
    WhatIfMove::new(0x20, "JASEN", CHARGED_FORWARD, Punch).enhanced(enhanced_names!("JASEN")),
    WhatIfMove::new(0x21, "OUGA", CHARGED_UP, Kick)
        .enhanced(enhanced_names!("OUGA"))
        .overhead(),
    WhatIfMove::new(0x1f, "SWITCH TO MANTIS", &[], TriplePunch).stance(false),
];

const fn gen_kit(specials: &'static [WhatIfMove], third: &'static str) -> WhatIfKit {
    WhatIfKit {
        specials,
        supers: ["ZAN'EI", "SHITENSHUU", third],
        v_skills: ["SOU-RYU", "KI-RYU"],
        v_triggers: ["ZETSUEI", "SHITENKETSU"],
    }
}
const GEN_MANTIS_SF3: WhatIfKit = gen_kit(GEN_MANTIS, "GENEI-JIN");
const GEN_CRANE_SF3: WhatIfKit = gen_kit(GEN_CRANE, "GENEI-JIN");
const GEN_MANTIS_KIT: WhatIfKit = gen_kit(GEN_MANTIS, "RYUKOHA");
const GEN_CRANE_KIT: WhatIfKit = gen_kit(GEN_CRANE, "RYUKOHA");

const BIRDIE_SPECIALS: &[WhatIfMove] = &[
    WhatIfMove::new(0x19, "BULL HEAD", CHARGED_FORWARD, Punch).enhanced(enhanced_names!("BULL HEAD")),
    WhatIfMove::new(0x1a, "BULL HORN", &[], ReleasePair),
    WhatIfMove::new(0x1b, "MURDERER CHAIN", SPIN, Punch).enhanced(enhanced_names!("MURDERER CHAIN")),
    WhatIfMove::new(0x20, "BAD HAMMER", &[DownForward], Punch).overhead(),
    WhatIfMove::new(0x21, "HANGING CHAIN", QCF, Kick).enhanced(enhanced_names!("HANGING CHAIN")),
];

const BIRDIE_KIT: WhatIfKit = WhatIfKit {
    specials: BIRDIE_SPECIALS,
    supers: ["THE BIRDIE", "BULL REVENGER", "BAD SPRAY"],
    v_skills: ["BREAK TIME", "CHEWING TIME"],
    v_triggers: ["ENJOY TIME", "BIRDIE TIME"],
};
const BIRDIE_SF4: WhatIfKit = WhatIfKit {
    supers: ["BAD SPRAY", "THE BIRDIE", "BULL REVENGER"],
    ..BIRDIE_KIT
};
const BIRDIE_SF6: WhatIfKit = WhatIfKit {
    supers: ["THE BIRDIE", "BULL REVENGER", "SKIP TO MY CHAIN"],
    ..BIRDIE_KIT
};

const EAGLE_KIT: WhatIfKit = WhatIfKit {
    specials: &[
        WhatIfMove::new(0x19, "CLOBBERING", HALF_CIRCLE, Punch),
        WhatIfMove::new(0x1a, "MANCHESTER BLACK", QCB, Punch).enhanced(enhanced_names!("MANCHESTER BLACK")),
        WhatIfMove::new(0x20, "CANTERBURY BLUE", QCF, Punch).enhanced(enhanced_names!("CANTERBURY BLUE")),
        WhatIfMove::new(0x21, "OXFORD RED", HALF_CIRCLE, Kick).enhanced(enhanced_names!("OXFORD RED")),
        WhatIfMove::new(0x22, "LIVERPOOL WHITE", QCB, Kick).overhead(),
        WhatIfMove::new(EAGLE_COUNTER, "ST ANDREWS GREEN", DP, Punch)
            .enhanced(enhanced_names!("ST ANDREWS GREEN")),
    ],
    supers: ["MANCHESTER GOLD", "UNION JACK", "ALBION TEMPEST"],
    v_skills: ["CANTERBURY BLUE", "ST ANDREWS GREEN"],
    v_triggers: ["UNION JACK", "ALBION TEMPEST"],
};

const ADON_KIT: WhatIfKit = WhatIfKit {
    specials: &[
        WhatIfMove::new(0x19, "RISING JAGUAR", DP, Kick).enhanced(enhanced_names!("RISING JAGUAR")),
        WhatIfMove::new(0x1b, "JAGUAR KICK", RDP, Kick).enhanced(enhanced_names!("JAGUAR KICK")),
        WhatIfMove::new(0x20, "JAGUAR TOOTH", HCB, Kick).enhanced(enhanced_names!("JAGUAR TOOTH")),
        WhatIfMove::new(0x21, "AIR JAGUAR KICK", &[Jump], Kick)
            .air()
            .overhead(),
    ],
    supers: [
        "JAGUAR VARIED ASSAULT",
        "JAGUAR REVOLVER",
        "JAGUAR AVALANCHE",
    ],
    v_skills: ["RISING JAGUAR", "JAGUAR TOOTH"],
    v_triggers: ["JAGUAR REVOLVER", "JAGUAR AVALANCHE"],
};

const TIGER_SHOT: WhatIfMove =
    WhatIfMove::new(0x1a, "TIGER SHOT", QCF, Punch).enhanced(enhanced_names!("TIGER SHOT"));
const LOW_TIGER_SHOT: WhatIfMove =
    WhatIfMove::new(0x1b, "GROUND TIGER SHOT", QCF, Kick).enhanced(enhanced_names!("GROUND TIGER SHOT"));
const TIGER_UPPERCUT: WhatIfMove =
    WhatIfMove::new(0x1f, "TIGER UPPERCUT", DP, Punch).enhanced(enhanced_names!("TIGER UPPERCUT"));
const TIGER_CRUSH: WhatIfMove =
    WhatIfMove::new(0x19, "TIGER KNEE CRUSH", DP, Kick).enhanced(enhanced_names!("TIGER KNEE CRUSH"));

const SAGAT_KIT: WhatIfKit = WhatIfKit {
    specials: &[TIGER_SHOT, LOW_TIGER_SHOT, TIGER_UPPERCUT, TIGER_CRUSH],
    supers: ["TIGER GENOCIDE", "TIGER CANNON", "TIGER RAID"],
    v_skills: ["ANGRY CHARGE", "HANUMAN CHARGE"],
    v_triggers: ["TIGER CHARGE", "TIGER ASSAULT"],
};
const SAGAT_SF6: WhatIfKit = WhatIfKit {
    specials: &[
        TIGER_SHOT,
        LOW_TIGER_SHOT,
        TIGER_UPPERCUT,
        TIGER_CRUSH,
        WhatIfMove::new(NEXUS, "TIGER NEXUS", QCB, Kick)
            .enhanced_at(OD_NEXUS, enhanced_names!("TIGER NEXUS")),
    ],
    supers: ["TIGER CANNON", "SAVAGE TIGER", "TIGER VANQUISHER"],
    ..SAGAT_KIT
};

pub(crate) fn kit(
    fighter: TournamentFighter,
    family: EraFamily,
    crane: bool,
) -> Option<&'static WhatIfKit> {
    Some(match fighter {
        Gen => match (family, crane) {
            (EraFamily::Sf3, false) => &GEN_MANTIS_SF3,
            (EraFamily::Sf3, true) => &GEN_CRANE_SF3,
            (_, false) => &GEN_MANTIS_KIT,
            (_, true) => &GEN_CRANE_KIT,
        },
        Birdie => match family {
            EraFamily::Sf4 => &BIRDIE_SF4,
            EraFamily::Sf6 => &BIRDIE_SF6,
            _ => &BIRDIE_KIT,
        },
        Eagle => &EAGLE_KIT,
        Adon => &ADON_KIT,
        Sagat if family == EraFamily::Sf6 => &SAGAT_SF6,
        Sagat => &SAGAT_KIT,
        _ => return None,
    })
}

pub(crate) fn super_input(
    fighter: TournamentFighter,
    family: EraFamily,
    slot: u8,
) -> Option<MoveInput> {
    if fighter != Sagat || family != EraFamily::Sf6 {
        return None;
    }
    Some(match slot {
        0x29 => MoveInput {
            motion: DOUBLE_QUARTER_CIRCLE,
            button: Punch,
        },
        0x2a => MoveInput {
            motion: QCB_TWICE,
            button: Kick,
        },
        0x2b => MoveInput {
            motion: DOUBLE_QUARTER_CIRCLE,
            button: Kick,
        },
        _ => return None,
    })
}

impl EraFighterState {
    pub(crate) fn gen_genei_jin(&self) -> bool {
        self.fighter == Some(Gen) && self.what_if_kit.install_frames != 0
    }

    pub(crate) fn what_if_others_projectile_hits(&self, slot: u8) -> Option<u8> {
        (self.what_if
            && self.fighter == Some(Sagat)
            && matches!(
                (self.style.family, slot),
                (EraFamily::Sf3, 0x2a) | (EraFamily::Sf6, 0x29)
            ))
        .then_some(5)
    }

    pub(crate) fn what_if_sagat_slot(&self, slot: u8) -> bool {
        self.what_if_active()
            && self.style.family == EraFamily::Sf6
            && self.fighter == Some(Sagat)
            && self.alpha_move_active(slot)
    }

    pub(crate) fn what_if_eagle_counter(&self) -> bool {
        self.what_if_active()
            && self.fighter == Some(Eagle)
            && self.alpha_move_active(EAGLE_COUNTER)
    }
}
