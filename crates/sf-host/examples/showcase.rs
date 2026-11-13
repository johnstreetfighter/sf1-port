//! Record a showcase reel from the headless game session, as `sf-game`
//! draws it: the Main Menu and Settings, a Local fight with the art pack and
//! the background bank (part of it through the ARCADE CABINET look, drawn
//! offscreen), and Training's move list with Show and the Try coach. Every
//! frame is the game's own; only the inputs are scripted. No sound.
//!
//! Writes `<out>/reel.mkv` (768x448 at 60 fps, lossless) and
//! `<out>/scenes.txt` (each caption's start in seconds, a tab, the caption).
//!
//! `cargo run -p sf-host --release --example showcase -- <out folder>`, then
//! `python -I tools/showcase/assemble.py <out folder> docs/media/showcase.mp4`
//! adds the key-art title and end cards and the captions.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use sf_engine::config::{MatchConfig, RulesetPreset};
use sf_engine::front::cabinet::flow_position;
use sf_engine::front::moves::Choice;
use sf_engine::front::pause::PauseBox;
use sf_engine::front::{FrameInputs, FrameOutput, GameSession, MenuItem, Panel, PanelSide};
use sf_engine::moves::{Strength, REGISTRY};
use sf_host::log::Log;
use sf_host::look::{Look, LookRequest, Loader};
use sf_host::picture::PictureRect;
use sf_video::render::{Frame, HEIGHT, WIDTH};
use sf_video::romset::{load_assets, Assets};

const ROMS: &str = "tools/reference/local/roms/sf";
const MANIFEST: &str = "tools/reference/rom-manifest.json";
const FIGHT_LOOP: u32 = 0x3eafa;
const OUT_W: u32 = WIDTH as u32 * 2;
const OUT_H: u32 = HEIGHT as u32 * 2;
/// The look is drawn at 4x and averaged down to 2x, as a screen shows it.
const LOOK_W: u32 = OUT_W * 2;
const LOOK_H: u32 = OUT_H * 2;

const RIGHT: usize = 0;
const LEFT: usize = 1;
const DOWN: usize = 2;
const UP: usize = 3;
const LP: usize = 0;
const MP: usize = 1;
const HP: usize = 2;
const LK: usize = 3;
const MK: usize = 4;
const HK: usize = 5;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("showcase"));
    std::fs::create_dir_all(&out).unwrap();
    let mut log = Log::create(&out.join("showcase.log"));
    let assets = load_assets(ROMS, MANIFEST).expect("local ROM set assembles");
    let art = sf_host::art::load_with_fallbacks(&assets, &[Path::new(ROMS)], &[Path::new("assets").to_path_buf()], &mut log).expect("the art pack loads");
    let stages = sf_host::stages::load(&assets, &mut log).expect("the background bank builds");

    let mut rec = Recorder::open(&out);
    let kit = Kit { art: &art.bank, stages: &stages };

    menus(&kit, &mut rec);
    local_fight(&kit, &mut rec);
    training(&kit, &mut rec);

    rec.finish(&out);
}

/// What every take is built from.
struct Kit<'a> {
    art: &'a sf_video::sprite_bank::SpriteBank,
    stages: &'a sf_video::background_bank::BackgroundBank,
}

impl Kit<'_> {
    fn take(&self) -> Take {
        let assets: Assets = load_assets(ROMS, MANIFEST).unwrap();
        let session = GameSession::new(assets, MatchConfig::from_preset(RulesetPreset::DefinitiveOverhaul))
            .with_sprite_bank(self.art)
            .with_background_bank(self.stages.clone());
        Take { session, recording: false }
    }
}

/// One session from power-on, stepped frame by frame.
struct Take {
    session: GameSession,
    recording: bool,
}

impl Take {
    fn step(&mut self, rec: &mut Recorder, input: FrameInputs) -> FrameOutput {
        let out = self.session.step(input);
        if self.recording {
            rec.frame(&out.frame, out.panel.as_ref());
        }
        out
    }

    /// Start recording from the next frame stepped.
    fn record(&mut self) {
        self.recording = true;
    }

    fn cut(&mut self, _rec: &mut Recorder) {
        self.recording = false;
    }
}

/// Where the reel is written: the frames to ffmpeg.
struct Recorder {
    ffmpeg: Child,
    frames: u64,
    scenes: Vec<(f64, String)>,
    gpu: Option<Gpu>,
    look_on: bool,
    rgb: Vec<u8>,
}

impl Recorder {
    fn open(out: &Path) -> Recorder {
        let ffmpeg = Command::new("ffmpeg")
            .args(["-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s"])
            .arg(format!("{OUT_W}x{OUT_H}"))
            .args(["-r", "60", "-i", "-", "-c:v", "libx264rgb", "-preset", "ultrafast", "-crf", "0"])
            .arg(out.join("reel.mkv"))
            .stdin(Stdio::piped())
            .spawn()
            .expect("ffmpeg on the PATH");
        Recorder {
            ffmpeg,
            frames: 0,
            scenes: Vec::new(),
            gpu: Gpu::open(),
            look_on: false,
            rgb: vec![0; (OUT_W * OUT_H * 3) as usize],
        }
    }

    fn caption(&mut self, text: &str) {
        self.scenes.push((self.frames as f64 / 60.0, text.to_owned()));
    }

    fn frame(&mut self, frame: &Frame, panel: Option<&Panel>) {
        match (&mut self.gpu, self.look_on) {
            (Some(gpu), true) => gpu.draw(frame, &mut self.rgb),
            _ => {
                for y in 0..OUT_H as usize {
                    for x in 0..OUT_W as usize {
                        let p = frame.argb[(y / 2) * WIDTH + x / 2];
                        let at = (y * OUT_W as usize + x) * 3;
                        self.rgb[at..at + 3].copy_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
                    }
                }
            }
        }
        if let Some(panel) = panel {
            self.panel(panel);
        }
        self.ffmpeg.stdin.as_mut().unwrap().write_all(&self.rgb).unwrap();
        self.frames += 1;
    }

    /// A panel (the move list's) drawn sharp at its edge of the frame at
    /// 2x, centred down, as the window draws it over the picture's side.
    fn panel(&mut self, panel: &Panel) {
        let (w, h) = (OUT_W as usize, OUT_H as usize);
        let (pw, ph) = (panel.width * 2, panel.height * 2);
        let x0 = match panel.side {
            PanelSide::Left => 0,
            PanelSide::Right => w.saturating_sub(pw),
        };
        let y0 = h.saturating_sub(ph) / 2;
        for y in 0..ph.min(h) {
            for x in 0..pw.min(w) {
                let p = panel.argb[(y / 2) * panel.width + x / 2];
                let at = ((y0 + y) * w + x0 + x) * 3;
                self.rgb[at..at + 3].copy_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
            }
        }
    }

    fn finish(mut self, out: &Path) {
        drop(self.ffmpeg.stdin.take());
        assert!(self.ffmpeg.wait().unwrap().success(), "ffmpeg wrote the reel");
        let scenes: String = self.scenes.iter().map(|(t, s)| format!("{t:.3}\t{s}\n")).collect();
        std::fs::write(out.join("scenes.txt"), scenes).unwrap();
        println!("{} frames ({:.1} s)", self.frames, self.frames as f64 / 60.0);
    }
}

/// The ARCADE CABINET look drawn offscreen, as the window draws it.
struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    look: Look,
    target: wgpu::Texture,
    readback: wgpu::Buffer,
}

impl Gpu {
    fn open() -> Option<Gpu> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
        let mut limits = wgpu::Limits::default();
        limits.max_inter_stage_shader_variables = adapter.limits().max_inter_stage_shader_variables;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("showcase"),
            required_features: sf_host::look::wanted_features() & adapter.features(),
            required_limits: limits,
            ..wgpu::DeviceDescriptor::default()
        }))
        .ok()?;
        let mut loader = Loader::default();
        let request = LookRequest { id: "arcade_cabinet".into(), preset: Path::new("shaders/looks/arcade-cabinet.slangp").into() };
        loader.request(&request, &device, &queue, &adapter.get_info());
        let (id, chain, _) = loader.wait()?;
        let chain = match chain {
            Ok(chain) => chain,
            Err(e) => {
                eprintln!("ARCADE CABINET did not load: {e}");
                return None;
            }
        };
        let look = Look::new(&id, chain, &device, wgpu::TextureFormat::Bgra8Unorm);
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("target"),
            size: wgpu::Extent3d { width: LOOK_W, height: LOOK_H, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(LOOK_W * LOOK_H * 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Some(Gpu { device, queue, look, target, readback })
    }

    fn draw(&mut self, frame: &Frame, rgb: &mut [u8]) {
        self.look.upload(&self.queue, frame);
        let view = self.target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let rect = PictureRect { origin: (0, 0), size: (LOOK_W, LOOK_H), scale: 4 };
        self.look.draw(&self.device, &self.queue, &mut encoder, &view, rect).expect("the look draws");
        encoder.copy_texture_to_buffer(
            self.target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(LOOK_W * 4), rows_per_image: Some(LOOK_H) },
            },
            self.target.size(),
        );
        self.queue.submit([encoder.finish()]);
        self.readback.slice(..).map_async(wgpu::MapMode::Read, |r| r.expect("map the readback"));
        self.device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).unwrap();
        {
            let bytes = self.readback.slice(..).get_mapped_range().expect("the mapped readback");
            let (w, lw) = (OUT_W as usize, LOOK_W as usize);
            for (i, out) in rgb.chunks_exact_mut(3).enumerate() {
                let (x, y) = (i % w * 2, i / w * 2);
                for (c, byte) in [2usize, 1, 0].into_iter().zip(out.iter_mut()) {
                    let at = |dx: usize, dy: usize| u32::from(bytes[((y + dy) * lw + x + dx) * 4 + c]);
                    *byte = ((at(0, 0) + at(1, 0) + at(0, 1) + at(1, 1) + 2) / 4) as u8;
                }
            }
        }
        self.readback.unmap();
    }
}

// ---- The takes -------------------------------------------------------------

/// The Main Menu, its cursor, and the Settings screen's tabs.
fn menus(kit: &Kit, rec: &mut Recorder) {
    let mut s = kit.take();
    idle_for(&mut s, rec, 200);
    tap(&mut s, rec, with(|i| i.menu.start = true));
    s.record();
    rec.caption("A new front end: Main Menu");
    idle_for(&mut s, rec, 50);
    for _ in 0..3 {
        tap(&mut s, rec, stick(DOWN));
        idle_for(&mut s, rec, 24);
    }
    tap(&mut s, rec, confirm());
    rec.caption("Settings: rules presets, controls, video and sound");
    idle_for(&mut s, rec, 70);
    for _ in 0..3 {
        tap(&mut s, rec, stick(RIGHT));
        idle_for(&mut s, rec, 50);
    }
    s.cut(rec);
}

/// Local: Lee against Mike on a stage from the background bank, with the
/// ARCADE CABINET look for a stretch of the fight.
fn local_fight(kit: &Kit, rec: &mut Recorder) {
    let mut s = kit.take();
    idle_for(&mut s, rec, 200);
    tap(&mut s, rec, with(|i| i.menu.start = true));
    tap(&mut s, rec, stick(DOWN));
    tap(&mut s, rec, confirm());
    tap(&mut s, rec, confirm());
    assert_eq!(s.session.screen(), "select", "Local's select");
    s.record();
    rec.caption("Player select with portraits");
    idle_for(&mut s, rec, 30);
    tap(&mut s, rec, stick(DOWN)); // P1 to Lee
    idle_for(&mut s, rec, 20);
    tap(&mut s, rec, confirm());
    tap(&mut s, rec, confirm()); // P1's default style
    for _ in 0..4 {
        idle_for(&mut s, rec, 10);
        tap(&mut s, rec, with(|i| i.p2.directions[RIGHT] = true)); // P2 from Ken to Mike
    }
    idle_for(&mut s, rec, 20);
    tap(&mut s, rec, with(|i| i.p2.buttons[LP] = true));
    s.step(rec, with(|i| i.p2.buttons[LP] = true)); // P2's default style
    let mut f = 0;
    while s.session.screen() != "stage select" {
        s.step(rec, idle());
        f += 1;
        assert!(f < 120, "the stage select (at {})", s.session.screen());
    }
    rec.caption("Stage select: every stage, each with its own theme");
    idle_for(&mut s, rec, 40);
    // The cursor starts on P2's stage (Mike's): over to Gen's and back to Lee's.
    for dir in [RIGHT, RIGHT, LEFT] {
        tap(&mut s, rec, stick(dir));
        idle_for(&mut s, rec, 24);
    }
    idle_for(&mut s, rec, 20);
    tap(&mut s, rec, confirm());
    rec.caption("Overhaul fighter art: new poses from each fighter's own sprites");
    let mut f = 0;
    while s.session.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        s.step(rec, idle());
        f += 1;
        assert!(f < 3_000, "the fight loop (at {})", s.session.screen());
    }
    let mut ko = None;
    for f in 0..1_200u32 {
        if f == 420 {
            rec.caption("Video looks: ARCADE CABINET, drawn by the game's own shaders");
            rec.look_on = true;
        }
        if f == 660 {
            rec.look_on = false;
        }
        s.step(rec, fight_script(f));
        let fighters = s.session.fight().unwrap().fight_read().fighters;
        if ko.is_none() && fighters.iter().any(|f| f.present && f.health == 0) {
            ko = Some(f);
        }
        // Through the round result, the winner named.
        if ko.is_some_and(|k| f >= k + 100) {
            break;
        }
    }
    rec.look_on = false;
    s.cut(rec);
}

/// Lee (P1) and Mike (P2): crouching attacks, jumping attacks and walks, on
/// a loop that keeps them close.
fn fight_script(f: u32) -> FrameInputs {
    with(|i| {
        let t = f % 300;
        // P1: walk in, crouch, crouching punch and kick, jump in with a kick.
        match t {
            0..=35 => i.p1.directions[RIGHT] = true,
            50..=110 => {
                i.p1.directions[DOWN] = true;
                if t == 70 {
                    i.p1.buttons[MP] = true;
                }
                if t == 95 {
                    i.p1.buttons[LK] = true;
                }
            }
            130..=134 => {
                i.p1.directions[UP] = true;
                i.p1.directions[RIGHT] = true;
            }
            150 => i.p1.buttons[HK] = true,
            200 => i.p1.buttons[HP] = true,
            230..=260 => i.p1.directions[LEFT] = true,
            _ => {}
        }
        // P2: the same, a beat behind.
        let u = (f + 150) % 300;
        match u {
            0..=30 => i.p2.directions[LEFT] = true,
            45..=100 => {
                i.p2.directions[DOWN] = true;
                if u == 62 {
                    i.p2.buttons[HP] = true;
                }
                if u == 88 {
                    i.p2.buttons[LP] = true;
                }
            }
            120..=124 => i.p2.directions[UP] = true,
            140 => i.p2.buttons[MP] = true,
            190 => i.p2.buttons[MK] = true,
            215..=240 => i.p2.directions[RIGHT] = true,
            _ => {}
        }
    })
}

/// Training (Ryu against the dummy): the move list, Show, and the Try
/// coach's call-outs, a clean input and then one too slow.
fn training(kit: &Kit, rec: &mut Recorder) {
    let mut s = kit.take();
    idle_for(&mut s, rec, 200);
    tap(&mut s, rec, with(|i| i.menu.start = true));
    tap(&mut s, rec, confirm()); // Single Player
    tap(&mut s, rec, stick(DOWN));
    tap(&mut s, rec, confirm()); // Training
    tap(&mut s, rec, stick(DOWN));
    tap(&mut s, rec, confirm()); // DUMMY
    assert_eq!(s.session.screen(), "select");
    tap(&mut s, rec, confirm()); // P1 Ryu
    tap(&mut s, rec, confirm()); // Ryu's default style
    tap(&mut s, rec, confirm()); // the dummy Ken
    s.step(rec, confirm()); // Ken's default style
    let mut f = 0;
    while s.session.fight_machine().map(flow_position) != Some(FIGHT_LOOP) {
        match s.session.screen() {
            "stage select" => tap(&mut s, rec, confirm()),
            "country select" => tap(&mut s, rec, with(|i| i.p1.buttons[LP] = true)),
            _ => {
                s.step(rec, idle());
            }
        }
        f += 1;
        assert!(f < 3_000, "Training's fight loop (at {})", s.session.screen());
    }
    idle_for(&mut s, rec, 200);
    s.record();
    rec.caption("Training: the move list, in the pause box");
    idle_for(&mut s, rec, 20);
    s.step(rec, with(|i| i.menu.start = true));
    s.step(rec, idle());
    let at = PauseBox::open(true).items().iter().position(|&i| i == MenuItem::Moves).expect("a MOVES row");
    idle_for(&mut s, rec, 30);
    for _ in 0..at {
        tap(&mut s, rec, stick(DOWN));
        idle_for(&mut s, rec, 8);
    }
    tap(&mut s, rec, confirm());
    idle_for(&mut s, rec, 70);
    let row = REGISTRY.iter().position(|e| e.name == "SHORYUKEN").unwrap();
    for _ in 0..row {
        tap(&mut s, rec, stick(DOWN));
        idle_for(&mut s, rec, 30);
    }
    tap(&mut s, rec, confirm());
    idle_for(&mut s, rec, 40);
    assert_eq!(s.session.move_list().unwrap().choosing(), Some(Choice::Show));
    rec.caption("Show: Ryu performs the move as its inputs light up");
    s.step(rec, confirm());
    let mut f = 0;
    while s.session.fight().unwrap().showing().is_some() || f < 10 {
        s.step(rec, idle());
        f += 1;
        assert!(f < 600, "the show ends");
    }
    idle_for(&mut s, rec, 60);
    rec.caption("Try: the coach watches your input and names what went wrong");
    for frame in recorded(row, 1) {
        s.step(rec, frame);
    }
    idle_for(&mut s, rec, 120);
    for frame in recorded(row, 12) {
        s.step(rec, frame);
    }
    idle_for(&mut s, rec, 150);
    for frame in recorded(row, 1) {
        s.step(rec, frame);
    }
    idle_for(&mut s, rec, 120);
    s.cut(rec);
}

/// The registry entry's input facing right at medium strength, each pad
/// frame held `slow` frames, then neutral.
fn recorded(row: usize, slow: usize) -> Vec<FrameInputs> {
    REGISTRY[row]
        .input
        .frames(true, Strength::Medium)
        .into_iter()
        .flat_map(|p| std::iter::repeat_n(with(|i| i.p1 = p), slow))
        .collect()
}

// ---- Inputs ----------------------------------------------------------------

fn idle() -> FrameInputs {
    FrameInputs::default()
}

fn with(f: impl Fn(&mut FrameInputs)) -> FrameInputs {
    let mut i = FrameInputs::default();
    f(&mut i);
    i
}

fn stick(dir: usize) -> FrameInputs {
    with(|i| i.p1.directions[dir] = true)
}

fn confirm() -> FrameInputs {
    with(|i| i.menu.confirm = true)
}

fn tap(s: &mut Take, rec: &mut Recorder, input: FrameInputs) {
    s.step(rec, input);
    s.step(rec, idle());
}

fn idle_for(s: &mut Take, rec: &mut Recorder, n: usize) {
    for _ in 0..n {
        s.step(rec, idle());
    }
}
