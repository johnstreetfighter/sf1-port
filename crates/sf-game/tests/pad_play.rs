//! Ticket #159 (`FE9`): the game driven by synthetic pad frames through
//! the host's input code. Two pads appear and drive the menus (Options,
//! the d-pad, Cross), the Local select (each pad its own side, Cross
//! confirming though it is also LK) and a fight; pulling P1's pad pauses
//! the fight with `P1 PAD DISCONNECTED`, the keyboard resumes it, P1 idles
//! until the pad returns, and the returned pad drives P1 again. Then the
//! Controls tab's rebind walk takes keys and pad buttons, refuses
//! duplicates with a notice, keeps a binding on Esc, and the result
//! survives the bindings file; `RESET TAB` brings the defaults back.
//! Local ROMs only; each test skips without them.

use std::path::Path;

use minifb::Key;
use sf_engine::config::MatchConfig;
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::settings::{Device, Settings};
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Transition};
use sf_engine::roster::TournamentFighter;
use sf_host::bindings::{BindingsTable, DEFAULT_BINDINGS};
use sf_host::input::frame_inputs_from;
use sf_host::pads::{PadButton, PadFrame, PadState};
use sf_video::compositor::{ink, small_glyph, RomFont};
use sf_video::decode::decode_char_tile;
use sf_video::render::{Frame, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "../../tools/reference/local/roms/sf";
const MANIFEST: &str = "../../tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;

fn assets() -> Option<Assets> {
    if !Path::new(ROMS).is_dir() {
        eprintln!("skipping: local ROM set not present");
        return None;
    }
    Some(load_assets(ROMS, MANIFEST).expect("local ROM set assembles"))
}

/// The test's hands: which pads are plugged in and what is held.
struct Rig {
    table: BindingsTable,
    plugged: [bool; 2],
    seen: [bool; 2],
}

impl Rig {
    fn new() -> Self {
        Rig {
            table: DEFAULT_BINDINGS,
            plugged: [false; 2],
            seen: [false; 2],
        }
    }

    fn plug(&mut self, n: usize) {
        self.plugged[n] = true;
        self.seen[n] = true;
    }

    /// One frame: `pad1` and `pad2` held on the plugged pads, `keys` held
    /// on the keyboard, `pressed` first pressed, `lost` pads pulled.
    fn frame(
        &self,
        pad1: &[PadButton],
        pad2: &[PadButton],
        keys: &[Key],
        pressed: Option<Key>,
        lost: [bool; 2],
    ) -> FrameInputs {
        let pads = PadFrame {
            pads: [
                self.plugged[0].then(|| PadState::holding(pad1)),
                self.plugged[1].then(|| PadState::holding(pad2)),
            ],
            seen: self.seen,
            lost,
            pressed: pad1.first().or(pad2.first()).copied(),
        };
        frame_inputs_from(&self.table, [Device::Auto; 2], &pads, |k| keys.contains(&k), pressed)
    }

    fn idle(&self) -> FrameInputs {
        self.frame(&[], &[], &[], None, [false; 2])
    }

    fn pad1(&self, held: &[PadButton]) -> FrameInputs {
        self.frame(held, &[], &[], None, [false; 2])
    }

    fn pad2(&self, held: &[PadButton]) -> FrameInputs {
        self.frame(&[], held, &[], None, [false; 2])
    }
}

/// `input` for one frame, then a frame of nothing held.
fn tap(s: &mut GameSession, rig: &Rig, input: FrameInputs) -> FrameOutput {
    let out = s.step(input);
    s.step(rig.idle());
    out
}

fn text_at(frame: &Frame, a: &Assets, col: usize, row: usize, text: &str, color: u8) -> bool {
    let font = RomFont::new(&a.gfx4, &a.maincpu);
    text.chars().enumerate().all(|(i, ch)| {
        let Some(code) = small_glyph(ch) else { return true };
        let tile = decode_char_tile(&a.gfx4, code as usize);
        (0..8).all(|ty| {
            (0..8).all(|tx| {
                let px = frame.argb[(row * 8 + ty) * WIDTH + (col + i) * 8 + tx];
                tile[ty][tx] == 3 || px == font.pen(color, tile[ty][tx])
            })
        })
    })
}

#[test]
fn two_pads_drive_the_menus_and_a_fight_and_a_pulled_pad_pauses_it() {
    let Some(a) = assets() else { return };
    let mut rig = Rig::new();
    let font_assets = assets().unwrap();
    let mut s = GameSession::new(a, MatchConfig::default()).with_settings(Settings::default());
    for _ in 0..200 {
        s.step(rig.idle());
    }
    rig.plug(0);
    rig.plug(1);
    use PadButton::*;
    assert_eq!(tap(&mut s, &rig, rig.pad1(&[Options])).screen, "main menu", "Options opens the menu");
    tap(&mut s, &rig, rig.pad1(&[DpadDown]));
    assert_eq!(tap(&mut s, &rig, rig.pad2(&[Cross])).screen, "multi player", "either pad's Cross confirms");
    assert_eq!(tap(&mut s, &rig, rig.pad1(&[Cross])).screen, "select");
    // Each pad drives its own cursor; Cross picks though it is also LK.
    let out = tap(&mut s, &rig, rig.pad1(&[Cross]));
    assert_eq!(out.transition, Some(Transition::Picked(TournamentFighter::Ryu)));
    let out = tap(&mut s, &rig, rig.pad2(&[Cross]));
    assert!(matches!(out.transition, Some(Transition::Fighters(..))), "{:?}", out.transition);
    while s.screen() != "country select" {
        s.step(rig.idle());
    }
    tap(&mut s, &rig, rig.pad1(&[Square])); // LP picks the flag
    let mut f = 0;
    while s.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(rig.idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.screen());
    }
    for _ in 0..30 {
        s.step(rig.idle());
    }
    // Cross and Circle are LK and MK in the fight, not the box's confirm
    // and back: no attack button opens it.
    for b in [Cross, Circle, Square, Triangle, R1, R2, L1, L2] {
        assert_eq!(tap(&mut s, &rig, rig.pad2(&[b])).screen, "fight", "P2's {b:?}");
        assert_eq!(tap(&mut s, &rig, rig.pad1(&[b])).screen, "fight", "P1's {b:?}");
    }

    // P1's cable is pulled: the box opens with the notice.
    rig.plugged[0] = false;
    let out = s.step(rig.frame(&[], &[], &[], None, [true, false]));
    assert_eq!((out.screen, out.transition), ("pause", Some(Transition::Paused)));
    let frame = s.fight_machine().unwrap().state.frame;
    let notice = "P1 PAD DISCONNECTED";
    let out = s.step(rig.idle());
    assert!(text_at(&out.frame, &font_assets, 10 + (28 - notice.len()) / 2, 8, notice, ink::YELLOW), "the notice is drawn");
    // P1 idles: the keyboard's P1 keys reach nothing.
    let held = rig.frame(&[], &[], &[Key::Right, Key::A], None, [false; 2]);
    assert_eq!((held.p1.directions, held.p1.buttons), ([false; 4], [false; 6]));
    // Any device resumes: the keyboard's Enter on RESUME.
    let out = tap(&mut s, &rig, rig.frame(&[], &[], &[Key::Enter], None, [false; 2]));
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Resume)));
    assert_eq!(s.screen(), "fight");
    for _ in 0..30 {
        s.step(rig.idle());
    }
    assert!(s.fight_machine().unwrap().state.frame > frame, "the fight runs on with P1 idle");
    assert_eq!(rig.idle().pads[0], None, "P1 has no pad");

    // The pad is plugged back in: it drives P1 again (its Options pauses).
    rig.plug(0);
    assert!(rig.idle().pads[0].is_some());
    let out = s.step(rig.pad1(&[Options]));
    assert_eq!((out.screen, out.transition), ("pause", Some(Transition::Paused)));
    let out = tap(&mut s, &rig, rig.pad2(&[Cross]));
    assert_eq!(out.transition, Some(Transition::Choose(MenuItem::Resume)), "P2's pad resumes");
}

#[test]
fn the_rebind_walk_binds_keys_and_pad_buttons_and_the_bindings_survive_the_file() {
    let Some(a) = assets() else { return };
    let mut rig = Rig::new();
    let mut s = GameSession::new(a, MatchConfig::default())
        .with_settings(Settings::default())
        .with_binds(DEFAULT_BINDINGS.binds(), DEFAULT_BINDINGS.binds());
    for _ in 0..200 {
        s.step(rig.idle());
    }
    rig.plug(0);
    use PadButton::*;
    tap(&mut s, &rig, rig.pad1(&[Options]));
    tap(&mut s, &rig, rig.pad1(&[DpadDown]));
    tap(&mut s, &rig, rig.pad1(&[DpadDown]));
    assert_eq!(tap(&mut s, &rig, rig.pad1(&[Cross])).screen, "settings");
    for _ in 0..4 {
        tap(&mut s, &rig, rig.pad1(&[R1])); // R1 switches tabs: to CONTROLS
    }
    // P1 DEVICE: AUTO, then KEYBOARD.
    tap(&mut s, &rig, rig.pad1(&[DpadRight]));
    // P2 DEVICE, P1 SCHEME, P2 SCHEME, MOVE INPUTS, then REBIND P1.
    for _ in 0..5 {
        tap(&mut s, &rig, rig.pad1(&[DpadDown]));
    }
    // But P1 is still on its pad until Settings is left.
    assert_eq!(tap(&mut s, &rig, rig.pad1(&[Cross])).screen, "rebind", "REBIND P1 opens the walk");

    let press = |s: &mut GameSession, input: FrameInputs| {
        let out = s.step(input);
        s.step(rig.idle());
        out
    };
    let key = |k: Key| rig.frame(&[], &[], &[k], Some(k), [false; 2]);
    // RIGHT: Esc keeps it.
    press(&mut s, key(Key::Escape));
    // LEFT: H is P2's right, refused; Esc keeps.
    let out = press(&mut s, key(Key::H));
    assert_eq!((out.screen, out.transition), ("rebind", None));
    press(&mut s, key(Key::Escape));
    // DOWN: Square is LP's pad button, refused; L2 binds.
    assert_eq!(press(&mut s, rig.pad1(&[Square])).transition, None);
    press(&mut s, rig.pad1(&[L2]));
    // UP kept; LP takes W; the rest kept, which ends the walk.
    press(&mut s, key(Key::Escape));
    press(&mut s, key(Key::W));
    for _ in 0..4 {
        press(&mut s, key(Key::Escape));
    }
    let out = s.step(key(Key::Escape));
    assert_eq!((out.screen, out.transition), ("settings", Some(Transition::Back)));
    assert!(out.bindings_changed);
    let binds = *s.binds().unwrap();
    assert_eq!((binds.p1[1].key, binds.p1[2].pad, binds.p1[4].key), ("Left", "L2", "W"));
    // The file keeps them.
    let table = DEFAULT_BINDINGS.with_binds(&binds).unwrap();
    assert_eq!(BindingsTable::from_toml(&table.to_toml()), Ok(table));
    assert_eq!(table.p1_pad.directions[2], L2);
    assert_eq!(table.p1.buttons[0], Key::W);

    // RESET TAB: the default bindings and devices.
    s.step(rig.idle());
    for _ in 0..3 {
        tap(&mut s, &rig, rig.pad1(&[DpadDown]));
    }
    let out = tap(&mut s, &rig, rig.pad1(&[Cross]));
    assert_eq!(out.transition, Some(Transition::ValueChanged));
    assert!(out.bindings_changed);
    assert_eq!(s.binds(), Some(&DEFAULT_BINDINGS.binds()));
    let out = tap(&mut s, &rig, rig.pad1(&[Circle]));
    assert!(out.settings_changed, "leaving Settings saves");
    assert_eq!(s.settings().devices, [Device::Auto; 2]);
}
