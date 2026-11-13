//! Training exercises driven by native pad inputs and fight observations.
use crate::era::{EraFamily, StylePick};
use crate::fair_cpu::Percept;
use crate::fight_reader::{FightRead, FighterRead};
use crate::front::PlayerInputs;
use crate::moves::{ButtonClass, InputNotation, MoveEntry, MoveKind, Strength};
use crate::roster::TournamentFighter;
use sf_video::callouts::{regions::STRIP_LETTERS, Strip};

use super::Drill;

impl Drill {
    pub const ALL: [Self; 4] = [
        Self::AntiAir,
        Self::WhiffPunish,
        Self::ThrowTech,
        Self::Survival,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::AntiAir => "ANTI-AIR",
            Self::WhiffPunish => "WHIFF PUNISH",
            Self::ThrowTech => "THROW TECH",
            Self::Survival => "DEFENSE",
        }
    }

    pub const fn target(self) -> u8 {
        if matches!(self, Self::Survival) {
            3
        } else {
            5
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrillController {
    drill: Drill,
    completed: u8,
    attempt: u16,
    phase: u16,
    defense_step: u8,
    defense_frames: u8,
    rest: u8,
    contacts: u8,
    was_active: bool,
}

crate::canonical::canonical_enum!(Drill, "TrainingDrill.v1";
    AntiAir => 0, WhiffPunish => 1, ThrowTech => 2, Survival => 3,
);
crate::canonical::canonical_struct!(DrillController, "DrillController.v1", {
    drill, completed, attempt, phase, defense_step, defense_frames, rest, contacts, was_active,
});

impl DrillController {
    pub const fn new(drill: Drill) -> Self {
        Self {
            drill,
            completed: 0,
            attempt: 0,
            phase: 0,
            defense_step: 0,
            defense_frames: 0,
            rest: 30,
            contacts: 0,
            was_active: false,
        }
    }

    pub fn drill(&self) -> Drill {
        self.drill
    }
    pub fn completed(&self) -> u8 {
        self.completed
    }

    pub(crate) fn input(&mut self, view: Percept, special: Option<MoveEntry>) -> PlayerInputs {
        if self.completed == self.drill.target()
            || !view.fighting
            || !view.me.present
            || !view.foe.present
        {
            return PlayerInputs::default();
        }
        if self.rest != 0 {
            self.rest -= 1;
            return PlayerInputs::default();
        }
        let toward = if view.foe.x >= view.me.x { 0 } else { 1 };
        let distance = view.me.x.abs_diff(view.foe.x);
        let spacing = match self.drill {
            Drill::AntiAir | Drill::Survival => 88,
            Drill::WhiffPunish => 72,
            Drill::ThrowTech => 30,
        };
        let mut pad = PlayerInputs::default();
        if self.phase == 0 {
            if !view.me.free() {
                return pad;
            }
            if distance.abs_diff(spacing) > 5 {
                pad.directions[if distance > spacing {
                    toward
                } else {
                    1 - toward
                }] = true;
                return pad;
            }
        }
        self.phase = self.phase.saturating_add(1);
        let phase = self.phase;
        match self.drill {
            Drill::AntiAir => return Self::jump_pad(toward, phase),
            Drill::WhiffPunish => {
                pad.buttons[if self.attempt % 2 == 0 { 2 } else { 5 }] = phase == 1
            }
            Drill::ThrowTech => {
                let tick = self.attempt % 2 == 1;
                if tick && phase == 1 {
                    pad.buttons[0] = true;
                } else if (!tick || phase > 14) && self.defense_step == 0 && view.me.free() {
                    pad.directions[toward] = true;
                    // The native ordinary throw is direction + heavy. The
                    // human can still tech it with the light pair.
                    if distance <= 35 && view.foe.free() && !view.foe.guarding() {
                        pad.buttons[2] = true;
                        self.defense_step = 1;
                    }
                }
            }
            Drill::Survival => return self.defense_input(view, toward, special),
        }
        pad
    }

    fn jump_pad(toward: usize, age: u16) -> PlayerInputs {
        let mut pad = PlayerInputs::default();
        if age <= 3 {
            pad.directions[toward] = true;
            pad.directions[3] = true;
        }
        pad.buttons[5] = (21..=23).contains(&age);
        pad
    }

    fn defense_input(
        &mut self,
        view: Percept,
        toward: usize,
        special: Option<MoveEntry>,
    ) -> PlayerInputs {
        let mut pad = PlayerInputs::default();
        let distance = view.me.x.abs_diff(view.foe.x);
        let step = self.defense_step;
        if step == 0 {
            let jump = Self::jump_pad(toward, self.phase);
            if self.phase >= 35 && view.me.free() {
                self.defense_step = 1;
            }
            return jump;
        }
        if step >= 4 {
            return pad;
        }
        if self.defense_frames == 0 {
            if !view.me.free() {
                return pad;
            }
            if distance > 38 {
                pad.directions[toward] = true;
                return pad;
            }
            self.defense_frames = 1;
        }
        let age = self.defense_frames;
        self.defense_frames = age.saturating_add(1);
        if step < 3 {
            if age <= 3 {
                pad.directions[2] = step == 1;
                pad.buttons[if step == 1 { 3 } else { 1 }] = true;
            }
            if age >= 12 && view.me.free() {
                self.defense_step += 1;
                self.defense_frames = 0;
            }
        } else if let Some(special) = special {
            let script = special.input.frames(toward == 0, Strength::Light);
            if let Some(frame) = script.get(usize::from(age - 1)) {
                return *frame;
            }
            self.defense_step = 4;
        } else {
            // Some borrowed styles expose no special to the command reader.
            // Keep the fourth defense contact available in those styles.
            pad.buttons[2] = age <= 3;
            self.defense_step = 4;
        }
        pad
    }

    pub(crate) fn observe(
        &mut self,
        before: FightRead,
        after: FightRead,
        before_tech: u8,
        after_tech: u8,
        parries: u8,
        random: u8,
    ) -> Option<&'static str> {
        if self.phase == 0 || self.rest != 0 || self.completed == self.drill.target() {
            return None;
        }
        let [player, opponent] = after.fighters;
        let [_, old_opponent] = before.fighters;
        self.was_active |= old_opponent.attack_active;
        let player_hit = after.landed[0].is_some_and(|hit| !hit.chip);
        let cpu_hit = after.landed[1].is_some_and(|hit| !hit.chip) && !after.blocked[1];
        let blocked = after.blocked[1] || after.landed[1].is_some_and(|hit| hit.chip);
        let time_out = self.phase
            >= if self.drill == Drill::Survival {
                220
            } else {
                95
            };
        let result = match self.drill {
            Drill::AntiAir if cpu_hit => Some(Err("ANTI-AIR EARLIER")),
            Drill::AntiAir if blocked => Some(Err("HIT IN THE AIR")),
            Drill::AntiAir if player_hit && airborne(old_opponent) => Some(Ok(())),
            Drill::AntiAir if time_out => Some(Err("HIT IN THE AIR")),
            Drill::WhiffPunish if cpu_hit || blocked || parries != 0 => Some(Err("BAIT THE WHIFF")),
            Drill::WhiffPunish
                if player_hit && (!self.was_active || old_opponent.attack_active) =>
            {
                Some(Err("BAIT THE WHIFF"))
            }
            Drill::WhiffPunish if player_hit && old_opponent.attacking() => Some(Ok(())),
            Drill::WhiffPunish if player_hit || time_out => Some(Err("PUNISH SOONER")),
            Drill::ThrowTech
                if before_tech > 0
                    && after_tech == 0
                    && player.move_id == 1
                    && opponent.move_id == 1 =>
            {
                Some(Ok(()))
            }
            Drill::ThrowTech if time_out || (before_tech > 0 && after_tech == 0) => {
                Some(Err("PRESS LP+LK"))
            }
            Drill::Survival if cpu_hit => Some(Err(if old_opponent.state & 4 != 0 {
                "HOLD DOWN-BACK"
            } else {
                "HOLD BACK"
            })),
            Drill::Survival => {
                if blocked || parries != 0 {
                    self.contacts |= 1 << self.defense_step.min(3);
                }
                if self.contacts.count_ones() == 4 {
                    Some(Ok(()))
                } else if time_out {
                    Some(Err("HOLD BACK"))
                } else {
                    None
                }
            }
            _ => None,
        }?;
        self.completed += u8::from(result.is_ok());
        self.attempt = self.attempt.wrapping_add(1);
        self.phase = 0;
        self.defense_step = 0;
        self.defense_frames = 0;
        self.rest = 30 + random % 21;
        self.contacts = 0;
        self.was_active = false;
        result.err()
    }

    pub(crate) fn strip(self) -> Strip {
        Strip::Input {
            steps: vec![
                self.drill.label().into(),
                format!("{}/{}", self.completed, self.drill.target()),
            ],
            lit: 1 + usize::from(self.completed == self.drill.target()),
        }
    }
}

fn airborne(body: FighterRead) -> bool {
    body.present && (matches!(body.move_id, 9..=11) || body.attacking() && body.state & 1 != 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RosterFighter {
    Tournament(TournamentFighter),
    EvilRyu,
    MBison,
}

impl From<TournamentFighter> for RosterFighter {
    fn from(value: TournamentFighter) -> Self {
        Self::Tournament(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Hit(u8),
    CrouchMediumKick,
    JumpHeavyKick,
    CustomCombo,
    FocusDash,
    VTrigger,
    Start(u8),
    HeavySpecial,
    Finish,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrialStep {
    pub label: &'static str,
    action: Action,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboTrialSpec {
    pub index: u8,
    pub steps: Vec<TrialStep>,
}

impl ComboTrialSpec {
    pub fn for_fighter_era(
        fighter: impl Into<RosterFighter>,
        family: EraFamily,
        index: u8,
    ) -> Self {
        use TournamentFighter::*;
        let fighter = fighter.into();
        let (normal, special, kick) = match fighter {
            RosterFighter::Tournament(Ryu | Ken) | RosterFighter::EvilRyu => {
                // The crouching recoil that allows the jump-in link rejects Tatsumaki.
                (0x0f, (0x1a, "HADOKEN"), (0x1a, "HADOKEN"))
            }
            RosterFighter::Tournament(Retsu) => (0x0d, (0x1a, "PALM"), (0x1a, "PALM")),
            RosterFighter::Tournament(Geki) => (0x11, (0x1a, "SHURIKEN"), (0x1a, "SHURIKEN")),
            RosterFighter::Tournament(Joe) => (0x11, (0x1b, "DYNAMITE"), (0x1b, "DYNAMITE")),
            RosterFighter::Tournament(Mike) => (0x11, (0x1a, "RUSH PUNCH"), (0x1a, "RUSH PUNCH")),
            RosterFighter::Tournament(Lee) => (0x11, (0x1a, "ZESSHOU"), (0x1a, "ZESSHOU")),
            RosterFighter::Tournament(Gen) => (0x11, (0x1a, "HYAKURENKO"), (0x1a, "HYAKURENKO")),
            RosterFighter::Tournament(Birdie) => (0x11, (0x1a, "BULL HEAD"), (0x1a, "BULL HEAD")),
            RosterFighter::Tournament(Eagle) => (0x11, (0x1a, "BATON"), (0x1a, "BATON")),
            RosterFighter::Tournament(Adon) => {
                (0x11, (0x19, "RISING JAGUAR"), (0x19, "RISING JAGUAR"))
            }
            RosterFighter::Tournament(Sagat) => (0x11, (0x1a, "TIGER SHOT"), (0x1b, "TIGER KNEE")),
            RosterFighter::MBison => (0x0f, (0x1a, "PSYCHO SHOT"), (0x1b, "KNEE PRESS")),
        };
        let hit = |id, label| TrialStep {
            label,
            action: Action::Hit(id),
        };
        let start = |id, label| TrialStep {
            label,
            action: Action::Start(id),
        };
        let jump = TrialStep {
            label: "J.HK",
            action: Action::JumpHeavyKick,
        };
        let medium = hit(
            normal,
            if normal == 0x0f {
                "ST.MP"
            } else if normal == 0x11 {
                "CR.LP"
            } else {
                "ST.LP"
            },
        );
        let low = TrialStep {
            label: "CR.MK",
            action: Action::CrouchMediumKick,
        };
        let primary = hit(special.0, special.1);
        let kick = hit(kick.0, kick.1);
        let heavy_special = TrialStep {
            label: "HEAVY SPECIAL",
            action: Action::HeavySpecial,
        };
        let finish = TrialStep {
            label: "SUPER",
            action: Action::Finish,
        };
        let extension = match family {
            EraFamily::Sf1987 | EraFamily::Sf1Plus => vec![medium, heavy_special],
            EraFamily::Sf2 => vec![medium, finish],
            EraFamily::SfZ => vec![
                TrialStep {
                    label: "CUSTOM",
                    action: Action::CustomCombo,
                },
                medium,
                primary,
            ],
            EraFamily::Sf3 => vec![hit(0x24, "EX"), finish],
            EraFamily::Sf4 => vec![
                primary,
                TrialStep {
                    label: "FADC",
                    action: Action::FocusDash,
                },
                TrialStep {
                    label: "ULTRA",
                    ..finish
                },
            ],
            EraFamily::Sf5 => vec![
                medium,
                TrialStep {
                    label: "V-TRIGGER",
                    action: Action::VTrigger,
                },
                primary,
            ],
            EraFamily::Sf6 => vec![
                medium,
                start(crate::era_fight::sf6::RUSH, "DRIVE RUSH"),
                low,
                primary,
            ],
        };
        let index = index.clamp(1, 5);
        let steps = match index {
            1 => vec![medium, primary],
            2 => vec![jump, low, kick],
            // Close HP has no cancel window in the native player tables.
            // The crouching medium kick gives a distinct heavy confirm.
            3 => vec![low, heavy_special],
            4 => extension,
            _ if matches!(family, EraFamily::Sf1987 | EraFamily::Sf1Plus) => {
                vec![jump, low, heavy_special]
            }
            _ => {
                let mut steps = vec![jump];
                steps.extend(extension);
                if steps
                    .last()
                    .is_none_or(|step| step.action != Action::Finish)
                {
                    steps.push(finish);
                }
                steps
            }
        };
        Self { index, steps }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TrialController {
    spec: ComboTrialSpec,
    progress: usize,
    contact: bool,
    last_count: u16,
    specials: Vec<u8>,
    commands: Vec<Option<MoveEntry>>,
}

#[derive(Clone, Copy)]
pub(crate) struct TrialFrame {
    pub read: FightRead,
    pub hits: u16,
    pub strength: u8,
    pub custom_combo: bool,
    pub focus_dash: bool,
    pub v_trigger: bool,
}

impl TrialController {
    pub fn new(mut spec: ComboTrialSpec, style: StylePick, entries: &[MoveEntry]) -> Self {
        let ground_special = |entry: &&MoveEntry| {
            entry.kind == MoveKind::Special
                && !entry.name.starts_with("AIR ")
                && !entry.name.contains(" AIR ")
                && !entry.name.contains("CHARGE")
        };
        let specials = entries
            .iter()
            .filter(ground_special)
            .map(|entry| entry.produces)
            .collect();
        let special = entries
            .iter()
            .filter(ground_special)
            .find(|entry| entry.produces == 0x1a)
            .or_else(|| entries.iter().find(ground_special));
        let super_move = entries
            .iter()
            .filter(|entry| entry.kind == MoveKind::Super)
            .next_back();
        let finisher = match style.family {
            EraFamily::Sf4 => entries.iter().find(|entry| {
                entry.kind == MoveKind::Super
                    && matches!(
                        entry.input.button,
                        ButtonClass::TriplePunch | ButtonClass::TripleKick
                    )
            }),
            _ => super_move,
        }
        .or(super_move)
        .or(special);
        let mut commands = Vec::with_capacity(spec.steps.len());
        spec.steps.retain_mut(|step| {
            let entry = match step.action {
                Action::Hit(id)
                    if id == crate::era_fight::sf3::EX_HADOKEN
                        && style.family == EraFamily::Sf3 =>
                {
                    entries
                        .iter()
                        .filter(ground_special)
                        .find(|entry| entry.name.starts_with("EX "))
                        .or(special)
                }
                Action::Hit(id) if id >= 0x19 => entries
                    .iter()
                    .filter(ground_special)
                    .find(|entry| entry.produces == id)
                    .or(special),
                Action::CustomCombo => {
                    let Some(entry) = entries.iter().find(|entry| entry.name == "CUSTOM COMBO")
                    else {
                        return false;
                    };
                    Some(entry)
                }
                Action::VTrigger => entries.iter().find(|entry| {
                    entry.kind == MoveKind::System && entry.input.button == ButtonClass::HeavyPair
                }),
                Action::Start(id) => entries
                    .iter()
                    .find(|entry| entry.kind == MoveKind::System && entry.produces == id),
                Action::Finish => finisher,
                _ => None,
            };
            if let Some(entry) = entry {
                step.action = match step.action {
                    Action::Start(_) => Action::Start(entry.produces),
                    Action::Hit(_) | Action::Finish => Action::Hit(entry.produces),
                    action => action,
                };
                if entry.kind == MoveKind::Special && !entry.name.starts_with(step.label) {
                    step.label = entry.name;
                }
            }
            commands.push(entry.copied());
            true
        });
        Self {
            spec,
            progress: 0,
            contact: false,
            last_count: 0,
            specials,
            commands,
        }
    }

    pub fn index(&self) -> u8 {
        self.spec.index
    }
    pub fn progress(&self) -> usize {
        self.progress
    }

    pub fn observe(&mut self, before: FightRead, frame: TrialFrame) -> Option<&'static str> {
        let Some(step) = self.spec.steps.get(self.progress) else {
            return None;
        };
        let after = frame.read;
        let hit = after.landed[0].filter(|hit| !hit.chip);
        let body = after.fighters[0];
        let combo_hit = hit
            .is_some_and(|hit| !self.contact || (hit.into_hitstun && frame.hits > self.last_count));
        let match_step = match step.action {
            Action::CustomCombo => frame.custom_combo,
            Action::FocusDash => frame.focus_dash && before.fighters[0].move_id != body.move_id,
            Action::VTrigger => frame.v_trigger,
            Action::Start(move_id) => {
                before.fighters[0].move_id != move_id && body.move_id == move_id
            }
            Action::Hit(move_id) => combo_hit && body.move_id == move_id,
            Action::CrouchMediumKick => combo_hit && body.move_id == 0x17 && frame.strength == 2,
            Action::JumpHeavyKick => {
                combo_hit && matches!(body.move_id, 0x23..=0x26) && frame.strength == 3
            }
            Action::HeavySpecial => {
                combo_hit && frame.strength == 3 && self.specials.contains(&body.move_id)
            }
            Action::Finish => false,
        };
        if match_step {
            self.progress += 1;
            if combo_hit
                && !matches!(
                    step.action,
                    Action::Start(_) | Action::CustomCombo | Action::FocusDash | Action::VTrigger
                )
            {
                self.contact = true;
                self.last_count = frame.hits.max(1);
            }
            return None;
        }
        if self.contact
            && (hit.is_some_and(|hit| !hit.into_hitstun) || !after.fighters[1].hitstun())
        {
            self.progress = 0;
            self.contact = false;
            self.last_count = 0;
            return Some("CANCEL FASTER");
        }
        None
    }

    pub fn strip(&self, notation: InputNotation, facing: bool) -> Strip {
        let labels: Vec<String> = self
            .spec
            .steps
            .iter()
            .zip(&self.commands)
            .map(|(step, entry)| {
                let Some(entry) = entry else {
                    return step.label.into();
                };
                if notation == InputNotation::Arrows {
                    return step.label.into();
                }
                if notation == InputNotation::Words {
                    return entry.input.words();
                }
                let button = entry.input.button.label();
                let motion = entry
                    .input
                    .motion
                    .iter()
                    .map(|dir| dir.written(notation, facing))
                    .collect::<Vec<_>>()
                    .join("");
                if motion.is_empty() {
                    step.label.into()
                } else {
                    format!("{motion} {button}")
                }
            })
            .collect();
        let active = self.progress.min(labels.len().saturating_sub(1));
        let mut first = 0;
        let mut width = 0;
        let mut end = 0;
        for (index, label) in labels.iter().enumerate() {
            width += label.chars().count() + 1;
            while width > STRIP_LETTERS + 1 && first < active.min(index) {
                width -= labels[first].chars().count() + 1;
                first += 1;
            }
            if width > STRIP_LETTERS + 1 && index > active {
                break;
            }
            end = index + 1;
        }
        Strip::Input {
            steps: labels[first..end]
                .iter()
                .map(|label| label.chars().take(STRIP_LETTERS).collect())
                .collect(),
            lit: self.progress - first,
        }
    }
}
