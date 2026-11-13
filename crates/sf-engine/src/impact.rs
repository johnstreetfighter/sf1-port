//! Presentation events captured at native combat contact.

/// The kind of collision that supplied the cue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImpactKind {
    HeavyStrike,
    MatSlam,
    WallSlam,
}

/// The affected fighter, in native fight slot order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImpactFighter {
    P1,
    /// P2 in versus play, the CPU opponent in Arcade, or the Dramatic boss.
    P2,
    Ally2,
}

impl ImpactFighter {
    pub(crate) const SLOTS: [Self; 3] = [Self::P1, Self::P2, Self::Ally2];
}

/// World-space direction of the impact, before any cabinet screen flip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImpactDirection {
    Left,
    Right,
    Down,
}

/// A contact detected on a specific native fight frame. Rollback and
/// spectator catch-up may deliver it later without changing that frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImpactCue {
    pub kind: ImpactKind,
    pub frame: u32,
    pub fighter: ImpactFighter,
    /// Stage-space x before camera scroll and visible-area crop.
    pub x: u16,
    /// Native sprite-space y before the visible-area crop.
    pub y: u16,
    pub direction: ImpactDirection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ImpactContact {
    pub(crate) kind: ImpactKind,
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) direction: ImpactDirection,
}

impl ImpactContact {
    pub(crate) fn cue(self, frame: u32, fighter: ImpactFighter) -> ImpactCue {
        let Self { kind, x, y, direction } = self;
        ImpactCue { kind, frame, fighter, x, y, direction }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::config::MatchConfig;
    use crate::front::cabinet::{Players, flow_position};
    use crate::front::fight::{Fight, FightDips, FightKind, FightSetup};
    use crate::front::session::PlayerInputs;
    use sf_video::romset::{Assets, load_assets};

    /// The real Local driver, with Ryu walked into standing normal range.
    pub(crate) fn close_local_fight() -> (Assets, Fight) {
        let assets = load_assets(
            "../../tools/reference/local/roms/sf", "../../tools/reference/rom-manifest.json",
        ).expect("impact tests require the verified local ROM set");
        let setup = FightSetup {
            kind: FightKind::Local,
            players: Players::Two,
            config: MatchConfig::definitive(),
            dsw1: FightDips::DSW1,
            dsw2: FightDips::default().dsw2(),
        };
        let mut fight = Fight::start(&assets.maincpu, setup, Some(0), None).unwrap();
        let idle = PlayerInputs::default();
        let mut frames = 0;
        while fight.driving() || flow_position(fight.machine()) != 0x3eafa {
            fight.step(&assets, idle, idle, false);
            frames += 1;
            assert!(frames < 1_500, "Local reaches the fight loop");
        }
        let right = PlayerInputs { directions: [true, false, false, false], ..idle };
        for _ in 0..120 {
            let o = &fight.machine().state.objectram;
            let x = |e: usize| u16::from_be_bytes([o[e + 6], o[e + 7]]);
            if x(0x400).abs_diff(x(0xa00)) <= 60 {
                for _ in 0..4 {
                    fight.step(&assets, idle, idle, false);
                }
                return (assets, fight);
            }
            fight.step(&assets, right, idle, false);
        }
        panic!("Ryu never walked into standing normal range");
    }
}
