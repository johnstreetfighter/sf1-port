//! A real fight frame to judge poses in (spec #164, AP4). A fresh machine
//! with the two fighters bound is driven by the cabinet driver to the
//! fight. Then the fighter's own 24 sprite entries are replaced by a pose,
//! laid out by the engine's grid code, so the game's renderer draws it on
//! the real stage, in the real palette, against the real opponent.

use sf_engine::config::MatchConfig;
use sf_engine::fighter::{pose_grid_entries, P1_STRUCT};
use sf_engine::front::cabinet::{drive_to_country_select, flow_position, pick_country, Players};
use sf_engine::roster::TournamentFighter;
use sf_engine::run::PowerOnMachine;
use sf_video::art::Pose;
use sf_video::romset::Assets;
use sf_video::scene::SceneState;

/// The fight loop's position in slot 1.
const FIGHT_LOOP: u32 = 0x3eafa;
/// Entries in a fighter's object region, and the bytes of one.
const ENTRIES: usize = 24;
const ENTRY: usize = 0x40;

/// One frame of a real fight, with where P1's fighter stands in it.
pub struct FightFrame {
    pub scene: SceneState,
    /// P1's entity: the object RAM offset of his first sprite entry.
    pub entity: usize,
    /// His centre and the ground line, in sprite coordinates.
    pub centre_x: i32,
    pub ground_y: i32,
    /// His sprite palette (attribute bits 0..3).
    pub palette: u16,
}

fn word(ram: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([ram[at], ram[at + 1]])
}

/// P1 `fighter` against `opponent` in Japan, a second into round 1,
/// standing in his stance.
pub fn fight_frame(rom: &Assets, fighter: TournamentFighter, opponent: TournamentFighter) -> Result<FightFrame, String> {
    let mut config = MatchConfig::default();
    config.roster.full_cast = true;
    config.roster.p1 = fighter;
    config.roster.p2 = opponent;
    let mut m = PowerOnMachine::new(rom.maincpu.clone()).with_match_config(config);
    drive_to_country_select(&mut m, Players::Two, 4_000).map_err(|e| format!("the drive: {e:?}"))?;
    pick_country(&mut m, &rom.maincpu, 0, 2_000).map_err(|e| format!("the flag: {e:?}"))?;
    let mut f = 0;
    while flow_position(&m) != FIGHT_LOOP {
        m.step();
        f += 1;
        if f > 3_000 {
            return Err("the fight loop never came".to_owned());
        }
    }
    for _ in 0..60 {
        m.step();
    }
    if !m.unrecovered().is_empty() {
        return Err(format!("untranslated {:05x?}", m.unrecovered()));
    }
    let ps = (P1_STRUCT - 0xff8000) as usize;
    let ram = m.work_ram();
    let entity = word(ram, ps + 0xe + 2) as usize;
    let scene = m.state.scene();
    let o = &scene.objectram;
    let (attr, y, x) = (word(o, entity + 2), word(o, entity + 4), word(o, entity + 6));
    let (rows, cols) = (o[entity + 0xc] as i32, o[entity + 0xd] as i32);
    let cell = if (attr as i16) >= 0x400 { 32 } else { 16 };
    Ok(FightFrame {
        centre_x: x as i32 + (cols + 1) * cell / 2,
        ground_y: y as i32 + (rows + 1) * cell,
        palette: attr & 0xf,
        entity,
        scene,
    })
}

impl FightFrame {
    /// The frame with P1's fighter showing `pose` at its anchor, facing
    /// right (or mirrored, facing left), every other sprite untouched.
    pub fn with_pose(&self, pose: &Pose, facing_left: bool) -> SceneState {
        let attr = self.palette | if pose.grid.large { 0x400 } else { 0 };
        let left = self.centre_x + pose.anchor[0] as i32;
        let top = self.ground_y + pose.anchor[1] as i32;
        let entries = pose_grid_entries(
            pose.grid.code,
            attr,
            pose.grid.rows,
            pose.grid.cols,
            left as u16,
            top as u16,
            facing_left,
        );
        let mut scene = self.scene.clone_state();
        let o = &mut scene.objectram;
        for i in 0..ENTRIES {
            let at = self.entity + i * ENTRY;
            // An entry the pose does not use is parked as the engine parks
            // it: its position zeroed, off screen.
            let words = entries.get(i).copied().unwrap_or([word(o, at), word(o, at + 2), 0, 0]);
            for (k, w) in words.iter().enumerate() {
                o[at + 2 * k..at + 2 * k + 2].copy_from_slice(&w.to_be_bytes());
            }
        }
        scene
    }
}

/// A copy of a scene (`SceneState` is not `Clone`).
trait CloneState {
    fn clone_state(&self) -> SceneState;
}

impl CloneState for SceneState {
    fn clone_state(&self) -> SceneState {
        SceneState {
            videoram: self.videoram.clone(),
            objectram: self.objectram.clone(),
            paletteram: self.paletteram.clone(),
            bg_scroll: self.bg_scroll,
            fg_scroll: self.fg_scroll,
            active: self.active,
            flip_screen: self.flip_screen,
            bg_page: self.bg_page,
            fg_page: self.fg_page,
        }
    }
}
