//! Twelve-fighter Arcade Mode (`#102`, `O11`).
//!
//! Any of the twelve tournament fighters can clear Japan, the USA, China,
//! England, and Thailand. Plank and barrel bonuses score the press when
//! `positive_edge_attacks` is on, and the release when it is off. Beating
//! Thailand opens that fighter's ending and a credits roll.

use crate::config::MatchConfig;
use crate::roster::TournamentFighter;
use crate::sf1plus::{CombatSession, MoveKind, Pad};
use sf_video::asset_pack::{validate_tile, IndexedTile, Rgba};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Country {
    Japan,
    Usa,
    China,
    England,
    Thailand,
}

pub const ROUTE: [Country; 5] = [
    Country::Japan,
    Country::Usa,
    Country::China,
    Country::England,
    Country::Thailand,
];

impl Country {
    pub const fn name(self) -> &'static str {
        match self {
            Country::Japan => "Japan",
            Country::Usa => "USA",
            Country::China => "China",
            Country::England => "England",
            Country::Thailand => "Thailand",
        }
    }
}

pub fn route_from(start: Country) -> &'static [Country] {
    let at = ROUTE
        .iter()
        .position(|country| *country == start)
        .unwrap_or(0);
    &ROUTE[at..]
}

pub fn rival(country: Country) -> TournamentFighter {
    match country {
        Country::Japan => TournamentFighter::Retsu,
        Country::Usa => TournamentFighter::Joe,
        Country::China => TournamentFighter::Lee,
        Country::England => TournamentFighter::Birdie,
        Country::Thailand => TournamentFighter::Sagat,
    }
}

fn opponent_for(player: TournamentFighter, country: Country) -> TournamentFighter {
    let waiting = rival(country);
    if player != waiting {
        return waiting;
    }
    match country {
        Country::Japan => TournamentFighter::Geki,
        Country::Usa => TournamentFighter::Mike,
        Country::China => TournamentFighter::Gen,
        Country::England => TournamentFighter::Eagle,
        Country::Thailand => TournamentFighter::Adon,
    }
}

pub fn player_intro(fighter: TournamentFighter) -> &'static str {
    match fighter {
        TournamentFighter::Ryu => "I came for the fight. Nothing else.",
        TournamentFighter::Ken => "Long flight. Make it worth the ticket.",
        TournamentFighter::Retsu => "Your hands are high. Mine are already in.",
        TournamentFighter::Geki => "You blinked. I was already gone.",
        TournamentFighter::Joe => "Keep the guard up. The kick goes through it.",
        TournamentFighter::Mike => "No dancing. I walk until you don't.",
        TournamentFighter::Lee => "One step. That is the whole punch.",
        TournamentFighter::Gen => "Still standing. That should worry you.",
        TournamentFighter::Birdie => "Head down. You can guess the rest.",
        TournamentFighter::Eagle => "Sticks are up. Watch the buttons.",
        TournamentFighter::Adon => "You wanted the knee. Here it is.",
        TournamentFighter::Sagat => "High shot or low. Both leave a mark.",
    }
}

pub fn rival_taunt(fighter: TournamentFighter) -> &'static str {
    match fighter {
        TournamentFighter::Ryu => "The dojo is not a tour.",
        TournamentFighter::Ken => "Loud entrance. Show the punch.",
        TournamentFighter::Retsu => "Japan. Front foot. Now.",
        TournamentFighter::Geki => "The shadow moves before you do.",
        TournamentFighter::Joe => "USA. Boots stay on.",
        TournamentFighter::Mike => "Small ring. You will find the rope.",
        TournamentFighter::Lee => "China. Eyes up.",
        TournamentFighter::Gen => "You are early.",
        TournamentFighter::Birdie => "England. I charge.",
        TournamentFighter::Eagle => "Two sticks. Count them.",
        TournamentFighter::Adon => "Thailand opens with a kick.",
        TournamentFighter::Sagat => "Last wall. The tiger is awake.",
    }
}

pub fn win_quote(fighter: TournamentFighter) -> &'static str {
    match fighter {
        TournamentFighter::Ryu => "Still walking.",
        TournamentFighter::Ken => "Round's mine. Hold the plane.",
        TournamentFighter::Retsu => "Quick hands. Slow feet.",
        TournamentFighter::Geki => "No echo.",
        TournamentFighter::Joe => "The legs got there first.",
        TournamentFighter::Mike => "Stay down. The bell rang.",
        TournamentFighter::Lee => "Told you. One step.",
        TournamentFighter::Gen => "Age hits harder than pride.",
        TournamentFighter::Birdie => "Horns down. Next town.",
        TournamentFighter::Eagle => "Sticks back in the case.",
        TournamentFighter::Adon => "The knee found you.",
        TournamentFighter::Sagat => "The shot missed. The knee did not.",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BonusKind {
    Plank,
    Barrel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerCheck {
    pub start: u8,
    pub end: u8,
    pub power_needed: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    pub frame: u8,
    pub down: bool,
    /// 1 light, 2 medium, 3 heavy. Zero while the button is up.
    pub power: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BonusStage {
    pub kind: BonusKind,
    pub positive_edge: bool,
    pub checks: Vec<PowerCheck>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BonusScore {
    pub kind: BonusKind,
    pub hits: u8,
    pub points: u16,
}

impl BonusStage {
    pub fn plank(positive_edge: bool) -> Self {
        Self {
            kind: BonusKind::Plank,
            positive_edge,
            checks: vec![PowerCheck {
                start: 8,
                end: 12,
                power_needed: 2,
            }],
        }
    }

    pub fn barrel(positive_edge: bool) -> Self {
        Self {
            kind: BonusKind::Barrel,
            positive_edge,
            checks: vec![
                PowerCheck {
                    start: 5,
                    end: 7,
                    power_needed: 1,
                },
                PowerCheck {
                    start: 12,
                    end: 14,
                    power_needed: 2,
                },
                PowerCheck {
                    start: 18,
                    end: 20,
                    power_needed: 3,
                },
            ],
        }
    }

    /// Score swings against the timing windows.
    ///
    /// Positive edge uses the frame the button goes down, and the strength on
    /// that frame. The 1987 check uses the release, and the strength that was
    /// held. A hold that began before the window does not count as a press
    /// inside it.
    pub fn judge(&self, samples: &[Sample]) -> BonusScore {
        let mut edges = Vec::new();
        let mut was_down = false;
        let mut held = 0u8;
        for sample in samples {
            let rising = sample.down && !was_down;
            let falling = !sample.down && was_down;
            if self.positive_edge && rising {
                edges.push((sample.frame, sample.power));
            }
            if !self.positive_edge && falling {
                edges.push((sample.frame, held));
            }
            if sample.down && sample.power > 0 {
                held = sample.power;
            }
            was_down = sample.down;
        }
        let mut used = vec![false; edges.len()];
        let mut hits = 0u8;
        let mut points = 0u16;
        for check in &self.checks {
            let found = edges.iter().enumerate().find(|(index, (frame, power))| {
                !used[*index]
                    && *frame >= check.start
                    && *frame <= check.end
                    && *power >= check.power_needed
            });
            if let Some((index, (_, power))) = found {
                used[index] = true;
                hits += 1;
                points = points.saturating_add(u16::from(*power) * 1000);
            }
        }
        BonusScore {
            kind: self.kind,
            hits,
            points,
        }
    }
}

/// A press that begins on `frame` and stays down. Positive-edge stages read this.
pub fn press_at(frame: u8, power: u8) -> Vec<Sample> {
    vec![
        Sample {
            frame: frame.saturating_sub(2),
            down: false,
            power: 0,
        },
        Sample {
            frame,
            down: true,
            power,
        },
        Sample {
            frame: frame.saturating_add(2),
            down: true,
            power,
        },
    ]
}

/// Held from frame 1 and let go on `frame`. Release-timed stages read this.
pub fn release_at(frame: u8, power: u8) -> Vec<Sample> {
    vec![
        Sample {
            frame: 1,
            down: true,
            power,
        },
        Sample {
            frame,
            down: false,
            power: 0,
        },
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ending {
    pub fighter: TournamentFighter,
    pub panels: Vec<IndexedTile>,
    pub credits: Vec<String>,
}

pub fn ending_for(fighter: TournamentFighter) -> Ending {
    let id = TournamentFighter::ALL
        .iter()
        .position(|one| *one == fighter)
        .unwrap_or(0);
    let ink = Rgba {
        r: 70 + (id as u8) * 12,
        g: 36 + (id as u8) * 6,
        b: 28,
        a: 255,
    };
    let accent = Rgba {
        r: 180,
        g: 140 - (id as u8 % 4) * 20,
        b: 40 + (id as u8) * 8,
        a: 255,
    };
    let mut pixels = vec![Rgba::TRANSPARENT; 64 * 16];
    for panel in 0..4 {
        let x0 = panel * 16;
        for y in 1..15 {
            for x in 1..15 {
                let mark = x == 1 + (id % 14) && y > 1 && y < 14;
                let draw = mark
                    || match panel {
                        0 => x > 4 && x < 11 && y > 2 && y < 13,
                        1 => x + (y / 2) > 6 && x < 13 && y > 3 && y < 14,
                        2 => y > 8 && x > 2 && x < 14,
                        _ => y == 3 || y == 12 || (x == 8 && y > 3 && y < 12),
                    };
                if draw {
                    let color = if (x + y + panel) % 2 == 0 {
                        ink
                    } else {
                        accent
                    };
                    pixels[y * 64 + x0 + x] = color;
                }
            }
        }
    }
    let panels = sf_video::asset_pack::quantize_sheet(&pixels, 64, 16).expect("ending sheet");
    for panel in &panels {
        validate_tile(panel).expect("ending panel");
    }
    Ending {
        fighter,
        panels,
        credits: vec![
            fighter.name().to_string(),
            "Thailand".to_string(),
            win_quote(fighter).to_string(),
            "Pictures".to_string(),
            "Sound".to_string(),
            "The end".to_string(),
        ],
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    pub country: Country,
    pub rival: TournamentFighter,
    pub player_line: &'static str,
    pub rival_line: &'static str,
    pub win_line: &'static str,
    pub bonus: BonusScore,
    pub specials: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArcadeClear {
    pub fighter: TournamentFighter,
    pub stops: Vec<Stop>,
    pub ending: Option<Ending>,
}

fn clearing_script(stage: &BonusStage) -> Vec<Sample> {
    if stage.positive_edge {
        match stage.kind {
            BonusKind::Plank => press_at(10, 3),
            BonusKind::Barrel => {
                let mut samples = press_at(6, 1);
                samples.extend(press_at(13, 2));
                samples.extend(press_at(19, 3));
                samples
            }
        }
    } else {
        match stage.kind {
            BonusKind::Plank => release_at(10, 3),
            BonusKind::Barrel => vec![
                Sample {
                    frame: 1,
                    down: true,
                    power: 1,
                },
                Sample {
                    frame: 6,
                    down: false,
                    power: 0,
                },
                Sample {
                    frame: 8,
                    down: true,
                    power: 2,
                },
                Sample {
                    frame: 13,
                    down: false,
                    power: 0,
                },
                Sample {
                    frame: 15,
                    down: true,
                    power: 3,
                },
                Sample {
                    frame: 19,
                    down: false,
                    power: 0,
                },
            ],
        }
    }
}

/// Walk `start` through Thailand. The first stop rehearses every special.
/// Later stops throw a jab so the route still steps a fight. Each stop then
/// runs the plank or barrel bonus with a swing timed for the active edge.
pub fn clear_route(
    fighter: TournamentFighter,
    config: &MatchConfig,
    start: Country,
) -> ArcadeClear {
    let positive_edge = config.toggles.positive_edge_attacks;
    let mut stops = Vec::new();
    for (i, country) in route_from(start).iter().copied().enumerate() {
        let rival = opponent_for(fighter, country);
        let mut specials = Vec::new();
        let mut session = CombatSession::from_config(fighter, rival, config);
        if i == 0 {
            let kit = session.p1_kit().clone();
            for mv in kit.moves.iter().filter(|mv| mv.kind == MoveKind::Special) {
                session.play(&mv.rehearsal());
                for _ in 0..mv.total_frames() {
                    session.step(Pad::default(), Pad::default());
                }
                if session.events.iter().any(|event| matches!(event, crate::sf1plus::CombatEvent::Started(name) if *name == mv.name)) {
                    specials.push(mv.name);
                }
                session = CombatSession::from_config(fighter, rival, config);
            }
        } else if let Some(jab) = session
            .p1_kit()
            .moves
            .iter()
            .find(|mv| mv.name == "stand LP")
            .cloned()
        {
            session.play(&jab.rehearsal());
        }
        let stage = if i % 2 == 0 {
            BonusStage::plank(positive_edge)
        } else {
            BonusStage::barrel(positive_edge)
        };
        let bonus = stage.judge(&clearing_script(&stage));
        stops.push(Stop {
            country,
            rival,
            player_line: player_intro(fighter),
            rival_line: rival_taunt(rival),
            win_line: win_quote(fighter),
            bonus,
            specials,
        });
    }
    let ending = stops
        .iter()
        .any(|stop| stop.country == Country::Thailand)
        .then(|| ending_for(fighter));
    ArcadeClear {
        fighter,
        stops,
        ending,
    }
}
