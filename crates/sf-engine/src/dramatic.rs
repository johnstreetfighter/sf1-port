//! 2v1 Dramatic Battle (`#110`, `O19`).
//!
//! Two allies and one boss step in the same frame, including their
//! projectiles. The camera keeps the outermost fighters inside a 384-pixel
//! view. The team can share one lifebar or keep two, and friendly fire is off
//! until the match asks for it. The boss starts with more health and hits harder.

use crate::canonical::{canonical_enum, canonical_struct};
use crate::era::Revision;
use crate::front::PlayerInputs;
use crate::roster::TournamentFighter;
use sf_video::compositor::{present, IndexedSprite, OverhaulScene, PresentMode};
use sf_video::render::{Frame, HEIGHT, WIDTH};

pub const VIEW_W: i32 = 384;
const ALLY_HEALTH: u16 = 1000;
const BOSS_HEALTH: u16 = 1600;
const ALLY_HIT: u16 = 40;
const BOSS_HIT: u16 = 70;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideMode {
    CoOp,
    CpuAlly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LifebarMode {
    #[default]
    Shared,
    Independent,
}

pub const ALLY_SUB_HP: u16 = 960;
pub const BOSS_SUB_HP: u16 = 1536;

/// Rules stamped on a native Dramatic fight. The roster holds Ally 1 and
/// the boss; the third fighter and stage are independent of those picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DramaticRules {
    pub co_op: bool,
    pub ally2: TournamentFighter,
    pub stage: TournamentFighter,
    pub lifebar: LifebarMode,
    pub friendly_fire: bool,
    /// Native slot order: Ally 1, boss, Ally 2.
    pub styles: [Revision; 3],
    /// The index of a rivalry preset, or None for custom picks.
    pub rivalry: Option<u8>,
}

impl DramaticRules {
    pub const fn supports_style(style: Revision) -> bool {
        matches!(style, Revision::Sf1Plus)
    }
}

/// Authoritative state attached to the translated machine. Health indices
/// use the same order as the three object-RAM fighter slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DramaticState {
    pub health: [u16; 3],
    pub boss_pad: PlayerInputs,
    pub boss_trackers: [u8; 9],
    pub boss_latches: [u8; 16],
}

impl DramaticState {
    pub fn reset_round(&mut self) {
        *self = Self { health: [ALLY_SUB_HP, BOSS_SUB_HP, ALLY_SUB_HP], ..Self::default() };
    }
}

canonical_enum!(LifebarMode, "dramatic-lifebar/v1"; Shared => 0, Independent => 1);
canonical_struct!(DramaticRules, "dramatic-rules/v1", {
    co_op, ally2, stage, lifebar, friendly_fire, styles, rivalry,
});
canonical_struct!(DramaticState, "dramatic-state/v1", {
    health, boss_pad, boss_trackers, boss_latches,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Ally,
    Boss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pad {
    pub left: bool,
    pub right: bool,
    pub attack: bool,
}

impl Default for Pad {
    fn default() -> Self {
        Self { left: false, right: false, attack: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Shot {
    x: i32,
    vx: i32,
    side: Side,
    owner: u8,
    alive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Body {
    fighter: TournamentFighter,
    x: i32,
    health: u16,
    color: u32,
    shot: Option<Shot>,
}

#[derive(Debug, Clone)]
pub struct DramaticBattle {
    allies: [Body; 2],
    boss: Body,
    pub mode: SideMode,
    pub lifebar: LifebarMode,
    pub friendly_fire: bool,
    shared: u16,
    pub camera: i32,
}

impl DramaticBattle {
    pub fn preset() -> Self {
        Self::new(
            TournamentFighter::Ryu,
            TournamentFighter::Ken,
            TournamentFighter::Sagat,
            SideMode::CoOp,
            LifebarMode::Shared,
            false,
        )
    }

    pub fn new(
        a: TournamentFighter,
        b: TournamentFighter,
        boss: TournamentFighter,
        mode: SideMode,
        lifebar: LifebarMode,
        friendly_fire: bool,
    ) -> Self {
        Self {
            allies: [
                Body { fighter: a, x: 80, health: ALLY_HEALTH, color: 0xff_d8_48_30, shot: None },
                Body { fighter: b, x: 130, health: ALLY_HEALTH, color: 0xff_e8_c0_40, shot: None },
            ],
            boss: Body { fighter: boss, x: 280, health: BOSS_HEALTH, color: 0xff_40_78_d0, shot: None },
            mode,
            lifebar,
            friendly_fire,
            shared: ALLY_HEALTH,
            camera: 0,
        }
    }

    pub fn shared(&self) -> u16 {
        self.shared
    }

    pub fn health(&self, index: usize) -> u16 {
        match index {
            0 | 1 => self.allies[index].health,
            _ => self.boss.health,
        }
    }

    pub fn x(&self, index: usize) -> i32 {
        match index {
            0 | 1 => self.allies[index].x,
            _ => self.boss.x,
        }
    }

    pub fn boss_max() -> u16 {
        BOSS_HEALTH
    }

    pub fn shot_x(&self, index: usize) -> Option<i32> {
        let body = match index {
            0 | 1 => &self.allies[index],
            _ => &self.boss,
        };
        body.shot.filter(|shot| shot.alive).map(|shot| shot.x)
    }

    /// One frame. In a CPU-ally match the second pad is ignored and that ally walks toward the boss.
    pub fn step(&mut self, a: Pad, mut b: Pad, boss: Pad) {
        if self.mode == SideMode::CpuAlly {
            b = Pad { left: false, right: self.allies[1].x < self.boss.x, attack: false };
        }
        self.drive(0, a);
        self.drive(1, b);
        self.drive(2, boss);
        self.fly_shots();
        self.clamp_view();
        self.resolve();
    }

    pub fn screen_x(&self, index: usize) -> i32 {
        self.x(index) - self.camera
    }

    pub fn frame(&self) -> Frame {
        let mut sprites = Vec::new();
        for (i, body) in self.allies.iter().chain(std::iter::once(&self.boss)).enumerate() {
            sprites.push(figure(self.screen_x(i), 150, body.color));
            if let Some(shot) = body.shot.filter(|shot| shot.alive) {
                sprites.push(figure(shot.x - self.camera, 156, 0xff_ff_f0_80));
            }
        }
        let hardware = Frame { argb: vec![0xff_18_14_28; WIDTH * HEIGHT] };
        present(
            PresentMode::Overhaul,
            &hardware,
            &OverhaulScene {
                sprites: &sprites,
                ..OverhaulScene::EMPTY
            },
        )
    }

    fn drive(&mut self, index: usize, pad: Pad) {
        let body = self.body_mut(index);
        if pad.left {
            body.x -= 6;
        }
        if pad.right {
            body.x += 6;
        }
        if pad.attack && body.shot.is_none() {
            let toward = if index == 2 { -1 } else { 1 };
            body.shot = Some(Shot {
                x: body.x + toward * 16,
                vx: toward * 8,
                side: if index == 2 { Side::Boss } else { Side::Ally },
                owner: index as u8,
                alive: true,
            });
        }
    }

    fn fly_shots(&mut self) {
        for index in 0..3 {
            if let Some(shot) = self.body_mut(index).shot.as_mut() {
                shot.x += shot.vx;
            }
        }
    }

    fn clamp_view(&mut self) {
        for _ in 0..4 {
            let xs = [self.allies[0].x, self.allies[1].x, self.boss.x];
            let min = *xs.iter().min().unwrap();
            let max = *xs.iter().max().unwrap();
            if max - min > VIEW_W - 16 {
                if self.allies[0].x == min {
                    self.allies[0].x += 6;
                }
                if self.allies[1].x == min {
                    self.allies[1].x += 6;
                }
                if self.boss.x == min {
                    self.boss.x += 6;
                }
                if self.allies[0].x == max {
                    self.allies[0].x -= 6;
                }
                if self.allies[1].x == max {
                    self.allies[1].x -= 6;
                }
                if self.boss.x == max {
                    self.boss.x -= 6;
                }
            }
        }
        let xs = [self.allies[0].x, self.allies[1].x, self.boss.x];
        let min = *xs.iter().min().unwrap();
        let max = *xs.iter().max().unwrap();
        self.camera = (min + max) / 2 - VIEW_W / 2;
        for index in 0..3 {
            let x = self.body_mut(index).x;
            let screen = x - self.camera;
            if screen < 8 {
                self.body_mut(index).x = self.camera + 8;
            } else if screen > VIEW_W - 8 {
                self.body_mut(index).x = self.camera + VIEW_W - 8;
            }
        }
    }

    fn resolve(&mut self) {
        let mut hits: Vec<(u8, u8, u16)> = Vec::new();
        for owner in 0..3u8 {
            let Some(shot) = self.shot_of(owner).filter(|shot| shot.alive) else {
                continue;
            };
            for target in 0..3u8 {
                if target == owner {
                    continue;
                }
                if (self.x_of(target) - shot.x).abs() > 14 {
                    continue;
                }
                let same = shot.side == Side::Ally && target < 2;
                if same && !self.friendly_fire {
                    continue;
                }
                let damage = if shot.side == Side::Boss { BOSS_HIT } else { ALLY_HIT };
                hits.push((owner, target, damage));
            }
        }
        for (owner, target, damage) in hits {
            self.apply_hit(target, damage);
            if let Some(shot) = self.body_mut(owner as usize).shot.as_mut() {
                shot.alive = false;
            }
        }
    }

    fn apply_hit(&mut self, target: u8, damage: u16) {
        if target < 2 && self.lifebar == LifebarMode::Shared {
            self.shared = self.shared.saturating_sub(damage);
            self.allies[0].health = self.shared;
            self.allies[1].health = self.shared;
            return;
        }
        let body = self.body_mut(target as usize);
        body.health = body.health.saturating_sub(damage);
    }

    fn body_mut(&mut self, index: usize) -> &mut Body {
        match index {
            0 => &mut self.allies[0],
            1 => &mut self.allies[1],
            _ => &mut self.boss,
        }
    }

    fn x_of(&self, index: u8) -> i32 {
        self.x(index as usize)
    }

    fn shot_of(&self, owner: u8) -> Option<Shot> {
        self.shot_x_body(owner).shot
    }

    fn shot_x_body(&self, owner: u8) -> &Body {
        match owner {
            0 => &self.allies[0],
            1 => &self.allies[1],
            _ => &self.boss,
        }
    }
}

fn figure(x: i32, y: i32, color: u32) -> IndexedSprite {
    let mut pixels = [0u8; 256];
    for py in 2..14 {
        for px in 4..12 {
            pixels[py * 16 + px] = 1;
        }
    }
    let mut palette = [0u32; 16];
    palette[1] = color;
    IndexedSprite { x, y, pixels, palette }
}
