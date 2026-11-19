//! Stun on hit and the dizzy sway (KS17, #234). The rules and values live
//! in [`crate::era_fight::stun`]; this reads the hit and the fighter's poses.

use super::Bus;
use crate::era_fight::stun::{ryu, StunHit, KESSEN_DIZZY_FRAMES, KESSEN_SWAY_FRAMES};
use crate::roster::TournamentFighter;

impl Bus<'_> {
    /// A Kessen attacker's clean hit adds his move's stun to the defender
    /// (`ps`), whatever the defender's style, and a full total dizzies him.
    /// An attacker in any other style deals no stun. The hit that ends a
    /// dizzy adds none: stun rests a while after every dizzy.
    pub(super) fn kessen_stun_hit(&mut self, owner: u32, ps: u32, me: u16, move_id: u8, projectile: bool) {
        let (Some(attacker), Some(defender)) = (self.era_side(owner), self.era_side(ps)) else { return };
        let fighters = &self.toggle_state.era.fighters;
        if !fighters[attacker].is_kessen() || fighters[attacker].fighter != Some(TournamentFighter::Ryu) {
            return;
        }
        let hit_index = fighters[attacker].stun.move_hits;
        let Some(stun) = self.kessen_ryu_stun(owner, me, move_id, projectile, hit_index) else { return };
        let fighters = &mut self.toggle_state.era.fighters;
        fighters[attacker].stun.move_hits = hit_index.saturating_add(1);
        let target = &mut fighters[defender];
        if target.dizzy_frames == 0 {
            target.stun.recovered();
        }
        if target.stun.add(stun) {
            target.dizzy_frames = KESSEN_DIZZY_FRAMES;
        }
    }

    /// A new move starts its own count of stun hits.
    pub(super) fn kessen_stun_move_started(&mut self, ps: u32) {
        if let Some(side) = self.era_side(ps) {
            self.toggle_state.era.fighters[side].stun.move_hits = 0;
        }
    }

    fn kessen_ryu_stun(&self, owner: u32, me: u16, move_id: u8, projectile: bool, hit_index: u8) -> Option<StunHit> {
        use crate::data_bank::st_ryu_normals::{
            CLOSE_HK, CLOSE_HP_HEADER, CLOSE_LK, CLOSE_MK, COLLARBONE_BREAKER, FAR_HK, FAR_HP, FAR_LK,
            FAR_MK, FAR_MP, SOLAR_PLEXUS_SMASHER,
        };
        let nth = |hits: [StunHit; 2]| hits[usize::from(hit_index.min(1))];
        if projectile {
            return match move_id {
                0x1a => Some(ryu::HADOKEN),
                crate::era_fight::SF2_EXTRA_SPECIAL => Some(ryu::RED_HADOKEN),
                crate::era_fight::SF2_SUPER => Some(ryu::SHINKU_HADOKEN),
                _ => None,
            };
        }
        let level = crate::combo::level(self.eb(me, 0x34));
        Some(match move_id {
            0x0d | 0x0e | 0x11..=0x13 | CLOSE_LK | FAR_LK => ryu::LIGHT,
            FAR_MP | CLOSE_MK | FAR_MK => ryu::MEDIUM,
            FAR_HP => {
                let close = self.data_bank.and_then(|bank| {
                    bank.era_move_header(TournamentFighter::Ryu, crate::era::EraFamily::Sf2, CLOSE_HP_HEADER)
                });
                if close.is_some() && self.st_ryu_standing_header(owner, me, FAR_HP) == close {
                    ryu::CLOSE_HEAVY_PUNCH
                } else {
                    ryu::FAR_HEAVY
                }
            }
            CLOSE_HK => nth(ryu::CLOSE_HEAVY_KICK),
            FAR_HK => ryu::FAR_HEAVY,
            0x1c..=0x1e if level >= 3 => ryu::CLOSE_HEAVY_PUNCH,
            0x1c..=0x1e => ryu::MEDIUM,
            0x17 if level >= 2 => ryu::MEDIUM,
            0x17 => ryu::LIGHT,
            0x18 => ryu::SWEEP,
            // Neutral jumps: punches 0x20, light and medium kicks 0x23, heavy kick 0x26.
            0x20 | 0x23 => match level {
                1 => ryu::JUMP_LIGHT,
                2 => ryu::JUMP_MEDIUM,
                _ => ryu::NEUTRAL_JUMP_HEAVY,
            },
            0x26 => ryu::NEUTRAL_JUMP_HEAVY,
            // Diagonal jumps: punches 0x21 / 0x22, kicks 0x24 / 0x25.
            0x21 | 0x22 => match level {
                1 => ryu::JUMP_LIGHT,
                2 => nth(ryu::DIAGONAL_JUMP_MEDIUM_PUNCH),
                _ => ryu::DIAGONAL_JUMP_HEAVY,
            },
            0x24 | 0x25 => match level {
                1 => ryu::JUMP_LIGHT,
                2 => ryu::JUMP_MEDIUM,
                _ => ryu::DIAGONAL_JUMP_HEAVY,
            },
            COLLARBONE_BREAKER | SOLAR_PLEXUS_SMASHER => ryu::COMMAND_NORMAL,
            crate::combo::throw::THROW_MOVE => ryu::THROW,
            0x19 => nth(ryu::SHORYUKEN),
            0x1b => ryu::TATSUMAKI,
            _ => return None,
        })
    }

    /// The dizzy picture, from poses the fighter already has: he stands in
    /// place and sways between his standing hit-reaction pose and his
    /// stance. (A stand-in: no fighter has dizzy art; see
    /// `docs/planning/art-upgrades.md`.) Called each frame of the dizzy
    /// lock, with the stance already running.
    pub(super) fn kessen_dizzy_sway(&mut self, me: u16, ps: u32) {
        let Some(side) = self.era_side(ps) else { return };
        let state = &self.toggle_state.era.fighters[side];
        if !state.stun.dizzied {
            return;
        }
        // The hit reaction that filled his stun has ended: stand him up.
        let move_id = self.eb(me, 0xa);
        if move_id != 1 && self.eb(me, 9) & 0x80 == 0 {
            self.start_move(me, ps, 1);
        }
        // The sway runs once he stands in it.
        if self.eb(me, 9) & 0xc1 != 0 || self.eb(me, 8) != 2 {
            return;
        }
        let elapsed = KESSEN_DIZZY_FRAMES.saturating_sub(self.toggle_state.era.fighters[side].dizzy_frames);
        if elapsed % KESSEN_SWAY_FRAMES != 0 {
            return;
        }
        if (elapsed / KESSEN_SWAY_FRAMES) % 2 == 0 {
            self.start_move(me, ps, 1);
            return;
        }
        let Some(frame) = self.standing_reaction_frame(ps) else { return };
        self.load_frame(me, frame);
        // Hold the pose for the sway's half; the stance stream resumes after.
        self.set_eb(me, 0xe, KESSEN_SWAY_FRAMES);
        // The reaction frame's invulnerability and travel are not his: a
        // dizzy fighter can be hit, and he stays where he is.
        self.set_eb(me, 0xf, self.eb(me, 0xf) & !0x08);
        self.set_eb(me, 0x14, 0);
        self.set_eb(me, 0x15, 0);
    }

    /// The frame the fighter's light standing hit reaction (move
    /// `0x31`) loads first, from the same tables the reaction reads (0x4461e
    /// for the players, 0x447c8 for the CPU opponent).
    fn standing_reaction_frame(&self, ps: u32) -> Option<u32> {
        const LIGHT_REACTION: u32 = (0x31 - 0x30) * 4;
        if self.is_player_driven(ps) {
            let (table, signed) = match self.bound_boss_tables(ps) {
                Some((alive, _)) => (alive, false),
                None if self.kens_side(ps) => (0x4473a, true),
                None => (0x44662, true),
            };
            let word = self.rw(table + LIGHT_REACTION);
            Some(if signed { word as i16 as i32 as u32 } else { u32::from(word) })
        } else {
            let mut d2 = u16::from(self.rb(0xffc003)) << 1;
            d2 = (d2 & 0xff00) | u16::from((d2 as u8).wrapping_add(self.rb(0xffc004)));
            d2 <<= 2;
            let table = self.rl(self.rl(0xffc050) + u32::from(d2));
            Some(u32::from(self.rw(table + LIGHT_REACTION)))
        }
        .filter(|&frame| frame != 0)
    }
}
