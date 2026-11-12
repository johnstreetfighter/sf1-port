//! The game's GPU window (ticket #205, VO2; ADR-0010): winit and wgpu,
//! presented on vsync (`Fifo`), windowed at a whole-number scale or
//! borderless fullscreen, switched in place. The picture is the 384×224
//! game frame at the largest whole-number vertical scale that fits, its
//! width set by the aspect ([`crate::picture`]), centred on black. Game frames are stepped by the pacing rule
//! ([`crate::pacing`]): one every *n* presents on a multiple of 60 Hz,
//! else a 60 Hz clock. The keyboard comes from the window's events
//! ([`crate::keys`]); pads stay with the game's own reader. The
//! development launcher keeps the minifb window ([`crate::window`]).
//!
//! The game drives the window through [`Driver`]: [`run`] owns the event
//! loop and calls [`Driver::step`] once per game frame, then shows
//! [`Driver::frame`], and applies [`Driver::video`] whenever it changes.
//!
//! With a look (ticket #207, VO4), the frame is drawn through its filter
//! chain into the picture rectangle ([`crate::look`]). [`Driver::look`]
//! names the look wanted; it loads off the render thread while the current
//! picture keeps presenting, and [`Driver::look_event`] hears whether it
//! loaded. A look that fails to load or to draw leaves the sharp picture.

use std::sync::Arc;
use std::time::{Duration, Instant};

use sf_video::render::{Frame, HEIGHT, WIDTH};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window, WindowId};

use sf_engine::front::settings::{Aspect, MAX_SCALE};
use sf_engine::front::{LookParam, LookTuning, Panel};

use crate::keys::Keyboard;
use crate::look::{wanted_features, Loader, Look, LookEvent, LookRequest};
use crate::pacing::{refresh_period, Pacer, Pacing, PresentStats};
use crate::panel::{beside_panel, PanelLayer, MIN_SURFACE_SIZE};
use crate::picture::{fit_scale, picture_rect, picture_width, PictureRect};

/// How the window is shown: windowed at `scale` times the game frame's
/// height (at most what fits the monitor), or borderless fullscreen on the
/// window's monitor, the picture in `aspect`, presented on vsync or not,
/// with the game stepped `frame_delay_ms` after each vblank.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VideoMode {
    pub scale: u32,
    pub fullscreen: bool,
    pub aspect: Aspect,
    pub vsync: bool,
    pub frame_delay_ms: u32,
}

/// What the window reports to the game each frame.
#[derive(Clone, Copy, Debug)]
pub struct WindowState {
    /// The surface's size in pixels.
    pub surface: (u32, u32),
    /// Where the picture is on the surface.
    pub picture: PictureRect,
    pub fullscreen: bool,
    /// The window's monitor's size in pixels and refresh rate, when it says.
    pub monitor: Option<(u32, u32)>,
    pub refresh_hz: Option<f64>,
    /// The largest windowed scale that fits the monitor in the current aspect.
    pub max_scale: u32,
    pub pacing: Pacing,
    pub stats: PresentStats,
    /// Time since the window opened.
    pub elapsed: Duration,
    #[cfg(windows)]
    win32_hwnd: Option<std::num::NonZeroIsize>,
}

impl WindowState {
    /// The parent window for native dialogs and clipboard operations.
    #[cfg(windows)]
    pub fn win32_hwnd(&self) -> Option<std::num::NonZeroIsize> {
        self.win32_hwnd
    }
}

/// The game behind the window.
pub trait Driver {
    /// Step one game frame with the keyboard as it stands. `false` quits.
    fn step(&mut self, keyboard: &mut Keyboard, window: &WindowState) -> bool;
    /// The game frame to show.
    fn frame(&self) -> &Frame;
    /// The mode the window should be in.
    fn video(&self) -> VideoMode;
    /// A line for the log (the adapter chosen, the refresh rate found).
    fn note(&mut self, _line: &str) {}
    /// The look to draw through; `None` is the sharp picture (`OFF`).
    fn look(&self) -> Option<&LookRequest> {
        None
    }
    /// Whether the look's file changed under the same id (a saved look
    /// replaced), once: it loads afresh, the old chain drawing until then.
    fn take_look_reload(&mut self) -> bool {
        false
    }
    /// A requested look loaded, or failed to load or draw.
    fn look_event(&mut self, _event: LookEvent) {}
    /// A look that loaded has these parameters (ticket #212), for the
    /// tuning screen.
    fn look_parameters(&mut self, _id: &str, _params: Vec<LookParam>) {}
    /// The tuning to set on the current look's chain; parameters it does
    /// not name take the look's own values.
    fn look_tuning(&self) -> Option<&LookTuning> {
        None
    }
    /// The tuning screen's or the move list's panel, drawn sharp over the
    /// look with the picture moved to the other edge; `None` when no panel
    /// shows.
    fn panel(&self) -> Option<&Panel> {
        None
    }
}

/// The refresh rate a monitor reports in millihertz. Windows reports
/// whole hertz and rounds the NTSC rates down (59.94 Hz as 59, 119.88 Hz
/// as 119), so a whole number one short of a multiple of 60 is taken as
/// that multiple times 1000/1001.
pub fn monitor_hz(millihertz: u32) -> Option<f64> {
    if millihertz == 0 {
        return None;
    }
    if millihertz % 1000 == 0 && (millihertz / 1000 + 1) % 60 == 0 {
        return Some(f64::from(millihertz / 1000 + 1) * 1000.0 / 1001.0);
    }
    Some(f64::from(millihertz) / 1000.0)
}

/// Open the window titled `title` in `video` and run `driver` until it
/// quits, the window is closed, or `limit` has passed (the smoke check).
pub fn run(title: &str, video: VideoMode, driver: &mut dyn Driver, limit: Option<Duration>) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| format!("event loop: {e}"))?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App {
        title: title.to_owned(),
        driver,
        video,
        limit,
        gpu: None,
        keyboard: Keyboard::focused(),
        pacer: Pacer::new(Pacing::Clock),
        idle: None,
        stats: PresentStats::default(),
        monitor: None,
        refresh_hz: None,
        start: Instant::now(),
        error: None,
        quit: false,
        loader: Loader::default(),
        look_id: None,
    };
    event_loop.run_app(&mut app).map_err(|e| format!("event loop: {e}"))?;
    match app.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

struct App<'d> {
    title: String,
    driver: &'d mut dyn Driver,
    video: VideoMode,
    limit: Option<Duration>,
    gpu: Option<Gpu>,
    keyboard: Keyboard,
    pacer: Pacer,
    /// The clock that steps the game while the window cannot present
    /// (minimised); gone once it presents again.
    idle: Option<Pacer>,
    stats: PresentStats,
    monitor: Option<(u32, u32)>,
    refresh_hz: Option<f64>,
    start: Instant,
    error: Option<String>,
    quit: bool,
    /// The look loading off the render thread, and the id last asked for.
    loader: Loader,
    look_id: Option<String>,
}

impl App<'_> {
    /// Start loading the look the driver wants when it changed; `OFF`
    /// drops the look at once.
    fn want_look(&mut self) {
        let reload = self.driver.take_look_reload();
        let wanted = self.driver.look();
        if !reload && wanted.map(|l| &l.id) == self.look_id.as_ref() {
            return;
        }
        self.look_id = wanted.map(|l| l.id.clone());
        let Some(gpu) = &mut self.gpu else { return };
        match wanted {
            None => {
                self.loader.cancel();
                gpu.look = None;
            }
            Some(request) if !reload && gpu.look.as_ref().is_some_and(|l| l.id == request.id) => self.loader.cancel(),
            Some(request) => self.loader.request(request, &gpu.device, &gpu.queue, &gpu.adapter_info),
        }
    }

    /// Take a finished load: the new look replaces the old, or the error
    /// goes to the driver.
    fn take_look(&mut self) {
        let Some((id, result, took)) = self.loader.poll() else { return };
        let params = self.loader.take_parameters().map(|(_, p)| p).unwrap_or_default();
        let Some(gpu) = &mut self.gpu else { return };
        let event = match result {
            Ok(chain) => {
                let mut look = Look::new(&id, chain, &gpu.device, gpu.config.format);
                look.set_defaults(&params);
                look.upload(&gpu.queue, self.driver.frame());
                gpu.look = Some(look);
                self.driver.look_parameters(&id, params);
                LookEvent::Loaded { id, took }
            }
            Err(error) => LookEvent::Failed { id, error },
        };
        self.look_event(event);
        self.tune_look();
    }

    /// Set the driver's tuning on the look's chain when it changed.
    fn tune_look(&mut self) {
        let Some(look) = self.gpu.as_mut().and_then(|g| g.look.as_mut()) else { return };
        let values = self.driver.look_tuning().filter(|t| t.look == look.id).map_or(&[][..], |t| &t.values[..]);
        look.tune(values);
    }

    fn look_event(&mut self, event: LookEvent) {
        let line = match &event {
            LookEvent::Loaded { id, took } => format!("look {id} loaded in {:.2} s", took.as_secs_f64()),
            LookEvent::Failed { id, error } => format!("look {id} failed, showing the sharp picture: {error}"),
        };
        self.driver.note(&line);
        self.driver.look_event(event);
    }

    fn state(&self) -> WindowState {
        let (surface, fullscreen) = match &self.gpu {
            Some(g) => ((g.config.width, g.config.height), g.window.fullscreen().is_some()),
            None => ((0, 0), false),
        };
        WindowState {
            surface,
            picture: picture_rect(surface, self.video.aspect),
            fullscreen,
            monitor: self.monitor,
            refresh_hz: self.refresh_hz,
            max_scale: fit_scale(self.monitor, self.video.aspect),
            pacing: self.pacer.pacing(),
            stats: self.stats,
            elapsed: self.start.elapsed(),
            #[cfg(windows)]
            win32_hwnd: self.gpu.as_ref().and_then(|gpu| {
                use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
                match gpu.window.window_handle().ok()?.as_raw() {
                    RawWindowHandle::Win32(handle) => Some(handle.hwnd),
                    _ => None,
                }
            }),
        }
    }

    /// Read the monitor's size and refresh rate, and choose pacing afresh
    /// if the rate changed (or `force`).
    fn choose_pacing(&mut self, force: bool) {
        let Some(gpu) = &self.gpu else { return };
        let monitor = gpu.window.current_monitor();
        self.monitor = monitor.as_ref().map(|m| (m.size().width, m.size().height));
        let hz = monitor.and_then(|m| m.refresh_rate_millihertz()).and_then(monitor_hz);
        if hz != self.refresh_hz || force {
            self.refresh_hz = hz;
            self.pacer = Pacer::for_display(self.video.vsync, hz);
            let rate = hz.map_or("unknown".to_owned(), |hz| format!("{hz:.2} Hz"));
            let line = format!("refresh {rate}: pacing {:?}", self.pacer.pacing());
            self.driver.note(&line);
        }
    }

    /// Step `steps` game frames; false once the game quits.
    fn step(&mut self, steps: u32) -> bool {
        for _ in 0..steps {
            let state = self.state();
            if !self.driver.step(&mut self.keyboard, &state) {
                self.quit = true;
                return false;
            }
        }
        let video = self.driver.video();
        if video != self.video {
            // Vsync switched: pacing is chosen afresh, so the 60 Hz clock
            // starts now and never from the window's opening.
            let repace = video.vsync != self.video.vsync;
            self.video = video;
            let max_scale = fit_scale(self.monitor, video.aspect);
            if let Some(gpu) = &mut self.gpu {
                gpu.apply(video, max_scale);
            }
            self.choose_pacing(repace);
        }
        self.want_look();
        self.tune_look();
        if steps > 0 {
            if let Some(gpu) = &mut self.gpu {
                gpu.set_panel(self.driver.panel());
            }
        }
        true
    }

    fn redraw(&mut self) {
        let Some(gpu) = &mut self.gpu else { return };
        if gpu.config.width == 0 || gpu.config.height == 0 {
            return;
        }
        // A look drawing or gone since the last present changes the queue.
        let latency = frame_latency(gpu.look.is_some());
        if latency != gpu.config.desired_maximum_frame_latency {
            gpu.config.desired_maximum_frame_latency = latency;
            gpu.reconfigure = true;
        }
        if std::mem::take(&mut gpu.reconfigure) {
            gpu.configure();
        }
        let texture = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                gpu.configure();
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Nothing shown this time: the game keeps time on the clock.
                let idle = self.idle.get_or_insert_with(|| Pacer::new(Pacing::Clock));
                let steps = idle.steps(self.start.elapsed());
                self.step(steps);
                return;
            }
        };
        if self.idle.take().is_some() {
            // Presenting again: pacing starts afresh.
            self.pacer = Pacer::new(self.pacer.pacing());
        }
        let acquired = Instant::now();
        let now = self.start.elapsed();
        let steps = self.pacer.steps(now);
        self.stats.present(now, steps, self.refresh_hz.and_then(refresh_period));
        // Frame delay: wait after the vblank, then read the keys and step,
        // so the input is as fresh as it can be when the frame is shown.
        if steps > 0 && self.video.vsync && self.video.frame_delay_ms > 0 {
            wait_until(acquired + Duration::from_millis(u64::from(self.video.frame_delay_ms)));
        }
        let worked = Instant::now();
        if !self.step(steps) {
            return;
        }
        self.take_look();
        let Some(gpu) = &mut self.gpu else { return };
        if steps > 0 {
            gpu.upload(self.driver.frame());
        }
        let drawn = gpu.draw(texture);
        self.stats.work(worked.elapsed());
        if let Err(error) = drawn {
            let id = self.gpu.as_mut().and_then(|g| g.look.take()).map(|l| l.id).unwrap_or_default();
            self.look_event(LookEvent::Failed { id, error });
        }
    }
}

/// Wait until `deadline`. The frame delay is a few milliseconds and Windows
/// sleeps in steps of 15, so the wait spins (yielding the thread).
fn wait_until(deadline: Instant) {
    while Instant::now() < deadline {
        std::thread::yield_now();
    }
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        match Gpu::open(event_loop, &self.title, self.video) {
            Ok(gpu) => {
                let line = format!("adapter {}", gpu.adapter);
                self.gpu = Some(gpu);
                self.driver.note(&line);
                self.choose_pacing(true);
                self.want_look();
                if let Some(gpu) = &self.gpu {
                    gpu.window.request_redraw();
                }
            }
            Err(e) => {
                self.error = Some(e);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size);
                }
                self.choose_pacing(false);
            }
            WindowEvent::Moved(_) | WindowEvent::ScaleFactorChanged { .. } => self.choose_pacing(false),
            WindowEvent::Focused(focused) => self.keyboard.focus(focused),
            WindowEvent::KeyboardInput { event, is_synthetic, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if pressed && !is_synthetic && self.keyboard.room_code_key(&event.logical_key, event.repeat) {
                    return;
                }
                self.keyboard.key(event.physical_key, pressed, event.repeat);
                if pressed {
                    self.keyboard.text(&event.logical_key, event.text.as_deref(), event.repeat);
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
                if let Some(gpu) = &self.gpu {
                    gpu.window.request_redraw();
                }
            }
            _ => {}
        }
        if self.quit || self.limit.is_some_and(|l| self.start.elapsed() >= l) {
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Minimised, the window gets no redraws: the game keeps its 60 Hz
        // clock (and the sound its frames) without spinning.
        let minimised = self.gpu.as_ref().is_some_and(|g| g.config.width == 0 || g.config.height == 0);
        if minimised {
            let idle = self.idle.get_or_insert_with(|| Pacer::new(Pacing::Clock));
            let steps = idle.steps(self.start.elapsed());
            self.step(steps);
            event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(4)));
        } else {
            event_loop.set_control_flow(ControlFlow::Poll);
        }
        if self.quit || self.limit.is_some_and(|l| self.start.elapsed() >= l) {
            event_loop.exit();
        }
    }
}

/// The picture shader: a triangle over the whole surface; each pixel
/// inside the picture shows the game pixel under it, the rest black. Rows
/// are always whole-number scaled. Across, a whole-number pixel width is
/// sharp; true 4:3's is the even average of the game pixels the surface
/// pixel covers (`blend`), so no game column is wider than another by more
/// than the smoothing.
const SHADER: &str = r#"
// rect is the picture's origin and size in surface pixels; blend is 1.0
// when the width is not a whole multiple of the frame's.
struct Picture { rect: vec4<f32>, blend: vec4<f32> };
@group(0) @binding(0) var frame: texture_2d<f32>;
@group(0) @binding(1) var<uniform> picture: Picture;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(i & 1u) * 4 - 1);
    let y = f32(i32(i & 2u) * 2 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let size = vec2<f32>(textureDimensions(frame));
    let at_picture = floor(at.xy) - picture.rect.xy;
    if (any(at_picture < vec2<f32>(0.0)) || any(at_picture >= picture.rect.zw)) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    // Sampled at the pixel's centre, so a division that is a hair off never
    // tips a whole-number result over an edge.
    let row = i32(floor((at_picture.y + 0.5) * size.y / picture.rect.w));
    let per_pixel = size.x / picture.rect.z;
    if (picture.blend.x == 0.0) {
        let col = i32(floor((at_picture.x + 0.5) * size.x / picture.rect.z));
        return vec4<f32>(textureLoad(frame, vec2<i32>(col, row), 0).rgb, 1.0);
    }
    // The span of game pixels under this surface pixel, at most two.
    let left = at_picture.x * size.x / picture.rect.z;
    let right = (at_picture.x + 1.0) * size.x / picture.rect.z;
    let first = floor(left);
    let edge = first + 1.0;
    let last = i32(size.x) - 1;
    let a = textureLoad(frame, vec2<i32>(min(i32(first), last), row), 0).rgb;
    if (right <= edge) {
        return vec4<f32>(a, 1.0);
    }
    let b = textureLoad(frame, vec2<i32>(min(i32(edge), last), row), 0).rgb;
    let share = (edge - left) / per_pixel;
    return vec4<f32>(mix(b, a, share), 1.0);
}
"#;

struct Gpu {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    picture: Picture,
    aspect: Aspect,
    adapter: String,
    adapter_info: wgpu::AdapterInfo,
    /// The present modes the surface offers.
    modes: Vec<wgpu::PresentMode>,
    /// The surface's present mode or queue depth changed.
    reconfigure: bool,
    /// The look drawing, when one is loaded; else the sharp picture.
    look: Option<Look>,
    /// The panel (the tuning screen's or the move list's), and whether it
    /// shows (the picture then at the other edge).
    panel: PanelLayer,
    panel_open: bool,
}

impl Gpu {
    fn open(event_loop: &ActiveEventLoop, title: &str, video: VideoMode) -> Result<Gpu, String> {
        let mut attributes = Window::default_attributes()
            .with_title(title)
            .with_min_inner_size(PhysicalSize::new(MIN_SURFACE_SIZE.0, MIN_SURFACE_SIZE.1));
        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::{IconExtWindows, WindowAttributesExtWindows};
            use winit::window::Icon;

            // The game binary embeds this resource. Other host clients can omit it.
            const ICON_RESOURCE: &str = "SF1_PORT_ICON";
            let caption = Icon::from_resource_name(ICON_RESOURCE, Some(PhysicalSize::new(16, 16)));
            let taskbar = Icon::from_resource_name(ICON_RESOURCE, None);
            attributes = attributes.with_window_icon(caption.ok()).with_taskbar_icon(taskbar.ok());
        }
        attributes = if video.fullscreen {
            attributes.with_fullscreen(Some(Fullscreen::Borderless(None)))
        } else {
            let monitor = event_loop.primary_monitor().map(|m| (m.size().width, m.size().height));
            attributes.with_inner_size(windowed_size(video.scale, video.aspect, fit_scale(monitor, video.aspect)))
        };
        let window = Arc::new(event_loop.create_window(attributes).map_err(|e| format!("window: {e}"))?);
        let instance = instance();
        let surface = instance.create_surface(window.clone()).map_err(|e| format!("surface: {e}"))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .map_err(|e| format!("graphics adapter: {e}"))?;
        let info = adapter.get_info();
        let adapter_name = format!("{} ({:?})", info.name, info.backend);
        let (device, queue) = device(&adapter)?;

        let caps = surface.get_capabilities(&adapter);
        // A plain (not sRGB) surface format, so the game's colours go out
        // as they are; with only sRGB ones, the frame texture is sRGB too
        // and the two conversions cancel.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .ok_or("the surface offers no format")?;
        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: Default::default(),
            width: size.width,
            height: size.height,
            present_mode: present_mode(video.vsync, &caps.present_modes),
            desired_maximum_frame_latency: frame_latency(false),
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        if size.width > 0 && size.height > 0 {
            surface.configure(&device, &config);
        }
        let picture = Picture::new(&device, format);
        picture.place(&queue, picture_rect((size.width, size.height), video.aspect));
        let panel = PanelLayer::new(&device, format);
        panel.place(&queue, (size.width, size.height));
        Ok(Gpu {
            window,
            surface,
            device,
            queue,
            config,
            picture,
            aspect: video.aspect,
            adapter: adapter_name,
            adapter_info: info,
            modes: caps.present_modes,
            reconfigure: false,
            look: None,
            panel,
            panel_open: false,
        })
    }

    /// Switch windowed and fullscreen, the windowed scale (up to
    /// `max_scale`), the aspect or vsync, in place.
    fn apply(&mut self, video: VideoMode, max_scale: u32) {
        let mode = present_mode(video.vsync, &self.modes);
        if mode != self.config.present_mode {
            // The surface is configured again before the next frame is
            // acquired: this runs while one is in flight.
            self.config.present_mode = mode;
            self.reconfigure = true;
        }
        let aspect_changed = std::mem::replace(&mut self.aspect, video.aspect) != video.aspect;
        if video.fullscreen {
            if self.window.fullscreen().is_none() {
                self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        } else {
            if self.window.fullscreen().is_some() {
                self.window.set_fullscreen(None);
            }
            if let Some(size) = self.window.request_inner_size(windowed_size(video.scale, video.aspect, max_scale)) {
                self.resize(size);
                return;
            }
        }
        // A resize places the picture itself; an aspect change alone does not.
        if aspect_changed {
            self.place();
        }
    }

    /// The picture's rectangle: centred, or at the left edge while the
    /// panel shows.
    fn rect(&self) -> PictureRect {
        let rect = picture_rect((self.config.width, self.config.height), self.aspect);
        if self.panel_open {
            beside_panel(rect, self.config.width, self.panel.side())
        } else {
            rect
        }
    }

    fn place(&self) {
        let surface = (self.config.width, self.config.height);
        self.picture.place(&self.queue, self.rect());
        self.panel.place(&self.queue, surface);
    }

    /// Show a panel (None: none shows).
    fn set_panel(&mut self, panel: Option<&Panel>) {
        let (size, side) = (self.panel.size(), self.panel.side());
        if let Some(panel) = panel {
            self.panel.upload(&self.device, &self.queue, panel);
        }
        let moved = panel.is_some() && (self.panel.size(), self.panel.side()) != (size, side);
        if panel.is_some() != self.panel_open || moved {
            self.panel_open = panel.is_some();
            self.place();
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.configure();
    }

    fn configure(&mut self) {
        if self.config.width > 0 && self.config.height > 0 {
            self.surface.configure(&self.device, &self.config);
            self.place();
        }
    }

    fn upload(&mut self, frame: &Frame) {
        self.picture.upload(&self.queue, frame);
        if let Some(look) = &mut self.look {
            look.upload(&self.queue, frame);
        }
    }

    /// Present the picture, through the look when there is one. An error
    /// from the look (its chain, or a wgpu validation error in its passes)
    /// comes back after the frame is presented sharp instead.
    fn draw(&mut self, texture: wgpu::SurfaceTexture) -> Result<(), String> {
        let view = texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("picture") });
        let mut failed = None;
        let rect = self.rect();
        if let Some(look) = &mut self.look {
            let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
            let mut through = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("look") });
            let drawn = look.draw(&self.device, &self.queue, &mut through, &view, rect);
            if self.panel_open {
                self.panel.draw(&mut through, &view);
            }
            let commands = through.finish();
            let error = pollster::block_on(scope.pop());
            match (drawn, error) {
                (Ok(()), None) => {
                    self.queue.submit([commands]);
                    self.window.pre_present_notify();
                    self.queue.present(texture);
                    return Ok(());
                }
                (Err(e), _) => failed = Some(e),
                (Ok(()), Some(e)) => failed = Some(format!("drawing the look: {e}")),
            }
        }
        self.picture.draw(&mut encoder, &view);
        if self.panel_open {
            self.panel.draw(&mut encoder, &view);
        }
        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.queue.present(texture);
        failed.map_or(Ok(()), Err)
    }
}

fn instance() -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    })
}

/// The device, with the features the looks use where the adapter has them
/// and the adapter's own limit on values passed between shader stages
/// (Guest's passes pass more than wgpu's default allows).
fn device(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue), String> {
    let mut limits = wgpu::Limits::default();
    limits.max_inter_stage_shader_variables = adapter.limits().max_inter_stage_shader_variables;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("sf-game"),
        required_features: wanted_features() & adapter.features(),
        required_limits: limits,
        ..wgpu::DeviceDescriptor::default()
    }))
    .map_err(|e| format!("graphics device: {e}"))
}

/// The game frame on the GPU and the pass that draws it as the picture on
/// a target of the format it was made for.
struct Picture {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    frame: wgpu::Texture,
    uniform: wgpu::Buffer,
    bytes: Vec<u8>,
}

impl Picture {
    fn new(device: &wgpu::Device, target: wgpu::TextureFormat) -> Picture {
        let format = if target.is_srgb() { wgpu::TextureFormat::Bgra8UnormSrgb } else { wgpu::TextureFormat::Bgra8Unorm };
        let frame = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("game frame"),
            size: wgpu::Extent3d {
                width: WIDTH as u32,
                height: HEIGHT as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("picture"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("picture"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("picture"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let view = frame.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("picture"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform.as_entire_binding(),
                },
            ],
        });
        Picture {
            pipeline,
            bind_group,
            frame,
            uniform,
            bytes: vec![0; WIDTH * HEIGHT * 4],
        }
    }

    /// Place the picture at `rect` on the target.
    fn place(&self, queue: &wgpu::Queue, rect: PictureRect) {
        let blend = if rect.crisp() { 0.0 } else { 1.0 };
        let values = [
            rect.origin.0 as f32,
            rect.origin.1 as f32,
            rect.size.0 as f32,
            rect.size.1 as f32,
            blend,
            0.0,
            0.0,
            0.0,
        ];
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        queue.write_buffer(&self.uniform, 0, &bytes);
    }

    /// Copy the game frame (ARGB words, so B, G, R, A in memory) to the GPU.
    fn upload(&mut self, queue: &wgpu::Queue, frame: &Frame) {
        for (out, px) in self.bytes.chunks_exact_mut(4).zip(&frame.argb) {
            out.copy_from_slice(&px.to_le_bytes());
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.frame,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH as u32 * 4),
                rows_per_image: Some(HEIGHT as u32),
            },
            wgpu::Extent3d {
                width: WIDTH as u32,
                height: HEIGHT as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Draw the picture centred on black over all of `view`.
    fn draw(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("picture"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

/// The present mode for vsync on or off among the `modes` the surface
/// offers: `Fifo` (always offered) on vsync; off, `Immediate`, else
/// `Mailbox`, else `Fifo`.
fn present_mode(vsync: bool, modes: &[wgpu::PresentMode]) -> wgpu::PresentMode {
    if vsync {
        return wgpu::PresentMode::Fifo;
    }
    [wgpu::PresentMode::Immediate, wgpu::PresentMode::Mailbox]
        .into_iter()
        .find(|m| modes.contains(m))
        .unwrap_or(wgpu::PresentMode::Fifo)
}

/// How many frames the swapchain may queue (ticket #214, VO11): three while
/// a look draws, else two. Measured on the 780M at 120 Hz in borderless
/// fullscreen: a look's chain runs on every other present, and with two
/// queued its present misses its refresh (about 94 presents a second
/// through `ARCADE CABINET`, though its GPU time fits); with three it holds
/// about 117. Running the chain on every present also held about 116, so
/// the GPU most likely slows between the bursts. One queued missed
/// refreshes even with the sharp picture (about 110 a second), so the
/// frame delay keeps the same depth.
fn frame_latency(look: bool) -> u32 {
    if look { 3 } else { 2 }
}

/// The windowed size at vertical `scale` (clamped to 1 and `max_scale`, the
/// largest that fits the monitor) in `aspect`, in pixels: exactly the
/// picture, with enough room for a panel at its native size.
fn windowed_size(scale: u32, aspect: Aspect, max_scale: u32) -> PhysicalSize<u32> {
    let scale = scale.clamp(1, max_scale.clamp(1, u32::from(MAX_SCALE)));
    PhysicalSize::new(picture_width(scale, aspect).max(MIN_SURFACE_SIZE.0), (HEIGHT as u32 * scale).max(MIN_SURFACE_SIZE.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_whole_hertz_ntsc_rates_are_read_as_ntsc() {
        assert_eq!(monitor_hz(120_000), Some(120.0));
        assert_eq!(monitor_hz(60_000), Some(60.0));
        assert_eq!(monitor_hz(144_000), Some(144.0));
        assert_eq!(monitor_hz(0), None);
        let ntsc = monitor_hz(59_000).unwrap();
        assert!((ntsc - 59.94).abs() < 0.01, "{ntsc}");
        assert_eq!(Pacing::choose(true, Some(ntsc)), Pacing::EveryNth(1));
        let ntsc = monitor_hz(119_000).unwrap();
        assert_eq!(Pacing::choose(true, Some(ntsc)), Pacing::EveryNth(2));
        // A precise rate is taken as it is.
        assert_eq!(monitor_hz(59_940), Some(59.94));
    }

    #[test]
    fn the_windowed_size_is_the_picture_at_a_whole_vertical_scale_up_to_what_fits() {
        assert_eq!(windowed_size(3, Aspect::Square, 4), PhysicalSize::new(1152, 672));
        assert_eq!(windowed_size(3, Aspect::True4x3, 4), PhysicalSize::new(896, 672));
        assert_eq!(windowed_size(3, Aspect::Integer4x3, 4), PhysicalSize::new(768, 672));
        assert_eq!(windowed_size(0, Aspect::Square, 4), PhysicalSize::new(384, 224));
        assert_eq!(windowed_size(1, Aspect::True4x3, 4), PhysicalSize::new(384, 224));
        // Past the old range of 4, to the largest that fits.
        assert_eq!(windowed_size(9, Aspect::Square, 4), PhysicalSize::new(1536, 896));
        assert_eq!(windowed_size(9, Aspect::Square, 8), PhysicalSize::new(3072, 1792));
        assert_eq!(windowed_size(8, Aspect::True4x3, 8), PhysicalSize::new(2389, 1792));
        assert_eq!(windowed_size(9, Aspect::True4x3, 20), PhysicalSize::new(2688, 2016));
    }

    #[test]
    fn vsync_off_takes_the_fastest_present_mode_the_surface_offers() {
        use wgpu::PresentMode::*;
        assert_eq!(present_mode(true, &[Fifo, Immediate]), Fifo);
        assert_eq!(present_mode(false, &[Fifo, Mailbox, Immediate]), Immediate);
        assert_eq!(present_mode(false, &[Fifo, Mailbox]), Mailbox);
        assert_eq!(present_mode(false, &[Fifo]), Fifo);
    }

    /// What the shader should draw at surface pixel (`x`, `y`) of the
    /// picture `rect`: black outside; the game row under it; across, the game
    /// pixel under it where the width is a whole multiple, else the even
    /// average of the (at most two) game pixels it covers, taken in linear
    /// light on an sRGB target (`linear`), which blends what it samples.
    fn expected(frame: &Frame, rect: PictureRect, linear: bool, x: i32, y: i32) -> [f64; 3] {
        let (px, py) = (x - rect.origin.0, y - rect.origin.1);
        if px < 0 || py < 0 || px >= rect.size.0 as i32 || py >= rect.size.1 as i32 {
            return [0.0; 3];
        }
        let row = py as usize / rect.scale as usize;
        let rgb = |col: usize| {
            let p = frame.argb[row * WIDTH + col.min(WIDTH - 1)];
            [f64::from((p >> 16) as u8), f64::from((p >> 8) as u8), f64::from(p as u8)]
        };
        let per = WIDTH as f64 / f64::from(rect.size.0);
        if rect.crisp() {
            return rgb(px as usize / (rect.size.0 as usize / WIDTH));
        }
        let (from, to) = (f64::from(px) * per, f64::from(px + 1) * per);
        let first = from.floor();
        let (a, b) = (rgb(first as usize), rgb(first as usize + 1));
        let share = (((first + 1.0).min(to) - from) / per).clamp(0.0, 1.0);
        let to_linear = |v: f64| {
            let v = v / 255.0;
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        let to_srgb = |v: f64| 255.0 * if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
        [0, 1, 2].map(|c| {
            if linear {
                to_srgb(to_linear(a[c]) * share + to_linear(b[c]) * (1.0 - share))
            } else {
                a[c] * share + b[c] * (1.0 - share)
            }
        })
    }

    /// The picture as the GPU draws it (offscreen, read back), in each
    /// aspect on plain and sRGB targets alike: black outside the rectangle;
    /// every game row whole-number scaled with no blending between rows; the
    /// whole-number widths exactly sharp; true 4:3's width the even average
    /// of the game pixels under each surface pixel, within rounding. Skips
    /// without a graphics adapter.
    #[test]
    fn the_gpu_draws_each_aspect_centred_on_black() {
        let instance = instance();
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            eprintln!("skipping: no graphics adapter");
            return;
        };
        let (device, queue) = device(&adapter).unwrap();
        let mut seed = 12345u32;
        let frame = Frame {
            argb: (0..WIDTH * HEIGHT)
                .map(|_| {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    0xff00_0000 | (seed >> 8)
                })
                .collect(),
        };
        // Widths that are multiples of 64 make a row of bytes the 256-byte
        // multiple a copy to a buffer needs. 832x500 is 2x tall in every
        // aspect; 1280x720 is 3x; 576x500 is too narrow for
        // 2x tall in every aspect, so each gives a scale back or narrows.
        for (w, h) in [(832u32, 500u32), (1280, 720), (576, 500)] {
            for aspect in Aspect::ALL {
                let rect = picture_rect((w, h), aspect);
                for format in [wgpu::TextureFormat::Bgra8Unorm, wgpu::TextureFormat::Rgba8Unorm, wgpu::TextureFormat::Bgra8UnormSrgb] {
                    let target = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("target"),
                        size: wgpu::Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                        view_formats: &[],
                    });
                    let readback = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("readback"),
                        size: u64::from(w * h * 4),
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                        mapped_at_creation: false,
                    });
                    let mut picture = Picture::new(&device, format);
                    picture.place(&queue, rect);
                    picture.upload(&queue, &frame);
                    let view = target.create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                    picture.draw(&mut encoder, &view);
                    encoder.copy_texture_to_buffer(
                        wgpu::TexelCopyTextureInfo {
                            texture: &target,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        wgpu::TexelCopyBufferInfo {
                            buffer: &readback,
                            layout: wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(w * 4),
                                rows_per_image: Some(h),
                            },
                        },
                        wgpu::Extent3d {
                            width: w,
                            height: h,
                            depth_or_array_layers: 1,
                        },
                    );
                    queue.submit([encoder.finish()]);
                    readback.slice(..).map_async(wgpu::MapMode::Read, |r| r.expect("map the readback"));
                    device
                        .poll(wgpu::PollType::Wait {
                            submission_index: None,
                            timeout: None,
                        })
                        .unwrap();
                    let bytes = readback.slice(..).get_mapped_range().expect("the mapped readback").to_vec();
                    let bgra = matches!(format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
                    let mut wrong = 0;
                    for y in 0..h as i32 {
                        for x in 0..w as i32 {
                            let at = ((y as u32 * w + x as u32) * 4) as usize;
                            let p = &bytes[at..at + 4];
                            let got = if bgra { [p[2], p[1], p[0]] } else { [p[0], p[1], p[2]] };
                            let want = expected(&frame, rect, format.is_srgb(), x, y);
                            // Sharp is exact; the blend may be a level or two off.
                            let slack = if rect.crisp() { 0.0 } else { 2.0 };
                            let off = (0..3).any(|c| (f64::from(got[c]) - want[c]).abs() > slack);
                            wrong += usize::from(off || p[3] != 0xff);
                        }
                    }
                    assert_eq!(wrong, 0, "{aspect:?} on {w}x{h} ({rect:?}), {format:?}: {wrong} pixels differ");
                }
            }
        }
    }
}
