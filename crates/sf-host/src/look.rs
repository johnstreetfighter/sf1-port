//! A look on the GPU (spec #203, ADR-0010): a RetroArch `.slangp` preset
//! run by librashader's wgpu runtime over the game frame into the picture.
//!
//! [`Loader`] loads a preset on a thread of its own, so the window keeps
//! presenting (with the look it has) until the new one is ready; a load
//! that fails, panics included, comes back as its error text. [`Look`]
//! draws: the filter chain renders the game frame into a picture texture
//! the size of the picture rectangle, only when the game frame or the
//! rectangle changed, and every present copies that picture onto the
//! target at the rectangle's origin, black around it. The rectangle is the
//! window's to compute (by aspect); the look only takes it.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use librashader::presets::{get_parameter_meta, ShaderFeatures, ShaderPreset};
use librashader::runtime::wgpu::{FilterChain, FilterChainOptions, WgpuOutputView};
use librashader::runtime::{FilterChainParameters, Size, Viewport};
use sf_engine::front::LookParam;
use sf_video::render::{Frame, HEIGHT, WIDTH};

/// The device features librashader's wgpu runtime uses when the adapter
/// has them (border clamping, the pipeline cache, float filtering). A
/// preset that needs one the adapter lacks fails to load and falls back.
pub fn wanted_features() -> wgpu::Features {
    wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER
        | wgpu::Features::PIPELINE_CACHE
        | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
        | wgpu::Features::FLOAT32_FILTERABLE
}

/// The look the game wants: its id and its preset file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookRequest {
    pub id: String,
    pub preset: PathBuf,
}

/// What became of a requested look.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LookEvent {
    /// It loaded in `took` and is drawing.
    Loaded { id: String, took: Duration },
    /// It failed to load or to draw; the window shows the sharp picture.
    /// `error` is the whole message, its first line the notice.
    Failed { id: String, error: String },
}

pub use crate::picture::PictureRect;

/// Load `preset` into a filter chain, blocking. Every failure (a parse
/// error, a missing file, a shader that does not compile, a wgpu
/// validation error, a panic inside librashader) comes back as text. Its
/// first line, the notice's, names the preset by its file name (a drop-in's
/// full path would fill the notice); the last line is the full path, for
/// the log.
pub fn load(preset: &Path, device: &wgpu::Device, queue: &wgpu::Queue, adapter: &wgpu::AdapterInfo) -> Result<FilterChain, String> {
    let name = preset.file_name().map_or_else(|| preset.display().to_string(), |n| n.to_string_lossy().into_owned());
    let failed = |what: String| format!("{name}: {what}\nin {}", preset.display());
    if !preset.is_file() {
        return Err(failed("not found".to_owned()));
    }
    // librashader's cache makes a wgpu pipeline cache whatever the device
    // has, and a device without the feature (DX12) panics on it.
    let options = FilterChainOptions {
        force_no_mipmaps: false,
        enable_cache: device.features().contains(wgpu::Features::PIPELINE_CACHE),
        adapter_info: Some(adapter.clone()),
    };
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let result = catch_unwind(AssertUnwindSafe(|| {
        FilterChain::load_from_path(preset, ShaderFeatures::NONE, device, queue, Some(&options))
    }));
    let validation = pollster::block_on(scope.pop());
    match (result, validation) {
        (Ok(Ok(chain)), None) => Ok(chain),
        (Ok(Ok(_)), Some(e)) => Err(failed(e.to_string())),
        (Ok(Err(e)), _) => Err(failed(e.to_string())),
        (Err(panic), _) => {
            let what = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panic.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_else(|| "a panic".to_owned());
            Err(failed(what))
        }
    }
}

/// The parameters of `preset` for the tuning screen (ticket #212): every
/// `#pragma parameter` its passes declare, with librashader's metadata and
/// the preset's own values as `initial`, in the order the passes declare
/// them (librashader's own list is unordered), section headings included.
pub fn parameters(preset: &Path) -> Result<Vec<LookParam>, String> {
    let parsed = ShaderPreset::try_parse(preset, ShaderFeatures::NONE).map_err(|e| format!("{}: {e}", preset.display()))?;
    let mut meta: Vec<_> = get_parameter_meta(&parsed).map_err(|e| format!("{}: {e}", preset.display()))?.collect();
    let mut order = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for pass in &parsed.passes {
        declared(&pass.path, &mut order, &mut seen);
    }
    let rank = |id: &str| order.iter().position(|o| o == id).unwrap_or(usize::MAX);
    meta.sort_by(|a, b| rank(&a.id).cmp(&rank(&b.id)).then_with(|| a.id.cmp(&b.id)));
    Ok(meta
        .into_iter()
        .map(|p| LookParam {
            id: p.id.to_string(),
            description: p.description,
            initial: p.initial,
            min: p.minimum,
            max: p.maximum,
            step: p.step,
        })
        .collect())
}

/// The parameter ids `source` declares, through its `#include`s, in order.
fn declared(source: &Path, order: &mut Vec<String>, seen: &mut std::collections::HashSet<PathBuf>) {
    if !seen.insert(source.to_path_buf()) {
        return;
    }
    let Ok(text) = std::fs::read_to_string(source) else { return };
    for line in text.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("#include") {
            if let Some(name) = rest.trim().strip_prefix('"').and_then(|r| r.split('"').next()) {
                let dir = source.parent().unwrap_or(Path::new("."));
                declared(&dir.join(name), order, seen);
            }
        } else if let Some(rest) = line.strip_prefix("#pragma parameter ") {
            if let Some(id) = rest.split_whitespace().next() {
                if !order.iter().any(|o| o == id) {
                    order.push(id.to_owned());
                }
            }
        }
    }
}

type Loaded = (String, Result<FilterChain, String>, Duration);

/// Loads one look at a time on a thread of its own. A newer request
/// supersedes an older one; a superseded load's result is dropped. A load
/// also reads the look's parameters ([`parameters`]), which
/// [`Loader::take_parameters`] hands over once the load is taken.
#[derive(Default)]
pub struct Loader {
    pending: Option<mpsc::Receiver<(Loaded, Vec<LookParam>)>>,
    parameters: Option<(String, Vec<LookParam>)>,
}

impl Loader {
    /// Start loading `request`.
    pub fn request(&mut self, request: &LookRequest, device: &wgpu::Device, queue: &wgpu::Queue, adapter: &wgpu::AdapterInfo) {
        let (tx, rx) = mpsc::channel();
        let id = request.id.clone();
        let (request, device, queue, adapter) = (request.clone(), device.clone(), queue.clone(), adapter.clone());
        let spawned = std::thread::Builder::new()
            .name("look loader".to_owned())
            .stack_size(16 << 20)
            .spawn(move || {
                let start = Instant::now();
                let result = load(&request.preset, &device, &queue, &adapter);
                let took = start.elapsed();
                // A look that loads but whose parameters cannot be read
                // still draws; the tuning screen shows no knobs.
                let params = match &result {
                    Ok(_) => parameters(&request.preset).unwrap_or_default(),
                    Err(_) => Vec::new(),
                };
                // Both have read the preset: a temporary one goes.
                crate::shaders::release(&request.preset);
                let _ = tx.send(((request.id, result, took), params));
            });
        self.pending = match spawned {
            Ok(_) => Some(rx),
            Err(e) => {
                // No thread: report the failure through the same channel.
                let (tx, rx) = mpsc::channel();
                let _ = tx.send(((id, Err(format!("the loader thread: {e}")), Duration::ZERO), Vec::new()));
                Some(rx)
            }
        };
    }

    /// Forget any load in progress.
    pub fn cancel(&mut self) {
        self.pending = None;
    }

    pub fn loading(&self) -> bool {
        self.pending.is_some()
    }

    /// The finished load, once: the id, the chain or the error, and how
    /// long it took.
    pub fn poll(&mut self) -> Option<Loaded> {
        let rx = self.pending.as_ref()?;
        match rx.try_recv() {
            Ok(done) => {
                self.pending = None;
                Some(self.took(done))
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                None
            }
        }
    }

    /// Wait for the load in progress (tests and the smoke check).
    pub fn wait(&mut self) -> Option<Loaded> {
        let done = self.pending.take()?.recv().ok()?;
        Some(self.took(done))
    }

    fn took(&mut self, (loaded, params): (Loaded, Vec<LookParam>)) -> Loaded {
        self.parameters = Some((loaded.0.clone(), params));
        loaded
    }

    /// The parameters of the look last taken from [`Loader::poll`] or
    /// [`Loader::wait`], once: its id and the list (empty when it failed).
    pub fn take_parameters(&mut self) -> Option<(String, Vec<LookParam>)> {
        self.parameters.take()
    }
}

/// The copy pass: each target pixel inside the rectangle shows the picture
/// pixel under it, the rest black. Written as linear on an sRGB target, so
/// the look's own gamma reaches the screen unchanged.
const COMPOSITE: &str = r#"
struct Place { origin: vec2<f32>, srgb: f32, pad: f32 };
@group(0) @binding(0) var picture: texture_2d<f32>;
@group(0) @binding(1) var<uniform> place: Place;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(i & 1u) * 4 - 1);
    let y = f32(i32(i & 2u) * 2 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

fn linear(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

@fragment
fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let p = floor(at.xy - place.origin);
    let size = vec2<f32>(textureDimensions(picture));
    if (any(p < vec2<f32>(0.0)) || any(p >= size)) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let c = textureLoad(picture, vec2<i32>(p), 0).rgb;
    return vec4<f32>(select(c, linear(c), place.srgb > 0.5), 1.0);
}
"#;

/// The format the look renders its picture in.
const PICTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;

/// A loaded look, drawing onto targets of one format.
pub struct Look {
    pub id: String,
    chain: FilterChain,
    /// The game frame as the chain reads it (plain, not sRGB: the presets
    /// do their own gamma). Copy-source for the presets' frame history.
    input: wgpu::Texture,
    bytes: Vec<u8>,
    /// The chain's output at the rectangle's size, and whether it shows
    /// the current game frame.
    picture: Option<(wgpu::Texture, wgpu::BindGroup)>,
    fresh: bool,
    frames: usize,
    pipeline: wgpu::RenderPipeline,
    place: wgpu::Buffer,
    srgb: bool,
    /// The look's own parameter values (from [`parameters`]), and the
    /// tuning last set on the chain over them.
    defaults: Vec<(String, f32)>,
    tuned: Vec<(String, f32)>,
}

impl Look {
    /// The look's own parameter values, which [`Look::tune`] sets back
    /// wherever a tuning does not name a parameter.
    pub fn set_defaults(&mut self, params: &[LookParam]) {
        self.defaults = params.iter().filter(|p| !p.heading()).map(|p| (p.id.clone(), p.initial)).collect();
    }

    /// Set `values` on the running chain (the tuning screen's changes), and
    /// every other parameter to the look's own value. The next draw runs
    /// the chain again; the same tuning twice does nothing.
    pub fn tune(&mut self, values: &[(String, f32)]) {
        if self.tuned == values {
            return;
        }
        let defaults = &self.defaults;
        self.chain.parameters().update_parameters(|map| {
            for (id, initial) in defaults {
                let value = values.iter().find(|(v, _)| v == id).map_or(*initial, |&(_, v)| v);
                map.insert(id.as_str().into(), value);
            }
            for (id, value) in values {
                map.insert(id.as_str().into(), *value);
            }
        });
        self.tuned = values.to_vec();
        self.fresh = false;
    }

    /// A parameter's value on the running chain.
    pub fn parameter(&self, id: &str) -> Option<f32> {
        self.chain.parameters().parameter_value(id)
    }
    /// The look `id` from its loaded `chain`, for targets in `target`.
    pub fn new(id: &str, chain: FilterChain, device: &wgpu::Device, target: wgpu::TextureFormat) -> Look {
        let input = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("look input"),
            size: wgpu::Extent3d {
                width: WIDTH as u32,
                height: HEIGHT as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("look composite"),
            source: wgpu::ShaderSource::Wgsl(COMPOSITE.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("look composite"),
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
        let place = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("look place"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Look {
            id: id.to_owned(),
            chain,
            input,
            bytes: vec![0; WIDTH * HEIGHT * 4],
            picture: None,
            fresh: false,
            frames: 0,
            pipeline,
            place,
            srgb: target.is_srgb(),
            defaults: Vec::new(),
            tuned: Vec::new(),
        }
    }

    /// Copy the game frame (ARGB words, so B, G, R, A in memory) to the
    /// chain's input; the next draw runs the chain again.
    pub fn upload(&mut self, queue: &wgpu::Queue, frame: &Frame) {
        for (out, px) in self.bytes.chunks_exact_mut(4).zip(&frame.argb) {
            out.copy_from_slice(&px.to_le_bytes());
        }
        queue.write_texture(
            self.input.as_image_copy(),
            &self.bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(WIDTH as u32 * 4),
                rows_per_image: Some(HEIGHT as u32),
            },
            self.input.size(),
        );
        self.fresh = false;
    }

    /// Record the look onto `target`: the chain into the picture when the
    /// game frame or the rectangle's size changed, then the picture at the
    /// rectangle's origin on black over all of `target`.
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        rect: PictureRect,
    ) -> Result<(), String> {
        let (w, h) = (rect.size.0.max(1), rect.size.1.max(1));
        let resized = self.picture.as_ref().is_none_or(|(t, _)| (t.width(), t.height()) != (w, h));
        if resized {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("look picture"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: PICTURE_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("look composite"),
                layout: &self.pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.place.as_entire_binding(),
                    },
                ],
            });
            self.picture = Some((texture, bind));
            self.fresh = false;
        }
        let (texture, bind) = self.picture.as_ref().expect("the picture was made above");
        if !self.fresh {
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let viewport = Viewport {
                x: 0.0,
                y: 0.0,
                mvp: None,
                output: WgpuOutputView::new_from_raw(&view, Size::new(w, h), PICTURE_FORMAT),
                size: Size::new(w, h),
            };
            self.chain
                .frame(&self.input, &viewport, encoder, self.frames, None)
                .map_err(|e| format!("drawing the look: {e}"))?;
            self.frames += 1;
            self.fresh = true;
        }
        let values = [rect.origin.0 as f32, rect.origin.1 as f32, if self.srgb { 1.0 } else { 0.0 }, 0.0];
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        queue.write_buffer(&self.place, 0, &bytes);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("look composite"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
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
        pass.set_bind_group(0, bind, &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}

/// The first line of a look's error, for the notice line.
pub fn first_line(error: &str) -> &str {
    error.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_shaders() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shaders")
    }

    /// The device as the window makes it, or `None` without an adapter.
    fn gpu() -> Option<(wgpu::Device, wgpu::Queue, wgpu::AdapterInfo)> {
        gpu_with(wgpu::Features::empty())
    }

    /// [`gpu`] with `extra` features too, where the adapter has them.
    fn gpu_with(extra: wgpu::Features) -> Option<(wgpu::Device, wgpu::Queue, wgpu::AdapterInfo)> {
        gpu_requesting(wanted_features() | extra)
    }

    /// A device with those of `features` the adapter has.
    fn gpu_requesting(features: wgpu::Features) -> Option<(wgpu::Device, wgpu::Queue, wgpu::AdapterInfo)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            eprintln!("skipping: no graphics adapter");
            return None;
        };
        let mut limits = wgpu::Limits::default();
        limits.max_inter_stage_shader_variables = adapter.limits().max_inter_stage_shader_variables;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("look test"),
            required_features: features & adapter.features(),
            required_limits: limits,
            ..wgpu::DeviceDescriptor::default()
        }))
        .expect("a device");
        Some((device, queue, adapter.get_info()))
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sf1-port-look-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Draw `look` over a test pattern onto an 832x500 target with the
    /// picture at `rect`, and read the target back as RGB.
    fn draw_offscreen(device: &wgpu::Device, queue: &wgpu::Queue, look: &mut Look, rect: PictureRect) -> Vec<[u8; 3]> {
        let (w, h) = (832u32, 500u32);
        let frame = Frame {
            argb: (0..WIDTH * HEIGHT)
                .map(|i| {
                    let (x, y) = (i % WIDTH, i / WIDTH);
                    // Bars of colour, a white grid every 16 pixels.
                    if x % 16 == 0 || y % 16 == 0 {
                        0xffff_ffff
                    } else {
                        [0xffe0_2020, 0xff20_e020, 0xff20_20e0, 0xffe0_e020][x * 4 / WIDTH]
                    }
                })
                .collect(),
        };
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
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(w * h * 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        look.upload(queue, &frame);
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        look.draw(device, queue, &mut encoder, &view, rect).expect("the look draws");
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(w * 4),
                    rows_per_image: Some(h),
                },
            },
            target.size(),
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
        bytes.chunks_exact(4).map(|p| [p[2], p[1], p[0]]).collect()
    }

    /// `ARCADE CABINET` loads off the render thread and draws the frame
    /// inside the picture rectangle only, black around it, and not as the
    /// plain picture: scanlines darken the rows between the beams. Skips
    /// without a graphics adapter.
    #[test]
    fn arcade_cabinet_loads_and_draws_into_the_picture_rectangle() {
        let Some((device, queue, info)) = gpu() else { return };
        let mut loader = Loader::default();
        let request = LookRequest {
            id: "arcade_cabinet".into(),
            preset: repo_shaders().join("looks/arcade-cabinet.slangp"),
        };
        loader.request(&request, &device, &queue, &info);
        assert!(loader.loading());
        let (id, chain, took) = loader.wait().expect("the loader answers");
        assert_eq!(id, "arcade_cabinet");
        let chain = chain.unwrap_or_else(|e| panic!("ARCADE CABINET loads: {e}"));
        eprintln!("ARCADE CABINET loaded in {:.2} s", took.as_secs_f64());
        let mut look = Look::new(&id, chain, &device, wgpu::TextureFormat::Bgra8Unorm);
        // 2x tall at true 4:3 would be 597x448; a 3x square 1152x672 is too
        // big for the target, so a 2x square picture at (32, 26).
        let rect = PictureRect { origin: (32, 26), size: (768, 448), scale: 2 };
        let pixels = draw_offscreen(&device, &queue, &mut look, rect);
        let (w, h) = (832i32, 500i32);
        let at = |x: i32, y: i32| pixels[(y * w + x) as usize];
        let mut outside_lit = 0;
        let mut inside_sum = 0u64;
        for y in 0..h {
            for x in 0..w {
                let inside = (32..32 + 768).contains(&x) && (26..26 + 448).contains(&y);
                let p = at(x, y);
                if inside {
                    inside_sum += p.iter().map(|&c| u64::from(c)).sum::<u64>();
                } else {
                    outside_lit += usize::from(p != [0, 0, 0]);
                }
            }
        }
        assert_eq!(outside_lit, 0, "black around the picture");
        let mean = inside_sum as f64 / (768.0 * 448.0 * 3.0);
        assert!(mean > 30.0, "the picture is lit (mean {mean:.1})");
        // Down a red bar's column, away from the grid: the brightest and
        // darkest rows differ clearly (scanlines), where the plain picture
        // is one flat red.
        let column: Vec<u8> = (26 + 40..26 + 440).map(|y| at(32 + 40 * 2 + 1, y)[0]).collect();
        let (lo, hi) = (*column.iter().min().unwrap(), *column.iter().max().unwrap());
        assert!(hi as i32 - lo as i32 > 40, "scanlines: red runs {lo}..{hi} down the column");
        // A second draw with no new frame reuses the picture; a new size
        // renders afresh.
        let again = draw_offscreen(&device, &queue, &mut look, PictureRect { origin: (0, 0), size: (597, 448), scale: 2 });
        assert_eq!(again[(100 * w + 700) as usize], [0, 0, 0], "outside a narrower picture");
    }

    /// Every built-in look loads and draws a lit picture inside the
    /// rectangle, black around it. Skips without a graphics adapter.
    #[test]
    fn every_built_in_look_loads_and_draws() {
        let Some((device, queue, info)) = gpu() else { return };
        for b in &crate::shaders::BUILT_IN {
            let chain = load(&repo_shaders().join(b.preset), &device, &queue, &info).unwrap_or_else(|e| panic!("{} loads: {e}", b.name));
            let mut look = Look::new(b.id, chain, &device, wgpu::TextureFormat::Bgra8Unorm);
            let rect = PictureRect { origin: (117, 26), size: (597, 448), scale: 2 };
            let pixels = draw_offscreen(&device, &queue, &mut look, rect);
            let inside = |i: usize| (117..117 + 597).contains(&(i % 832)) && (26..26 + 448).contains(&(i / 832));
            let outside_lit = pixels.iter().enumerate().filter(|&(i, p)| !inside(i) && *p != [0, 0, 0]).count();
            assert_eq!(outside_lit, 0, "{}: black around the picture", b.name);
            let sum: u64 = pixels.iter().enumerate().filter(|&(i, _)| inside(i)).map(|(_, p)| p.iter().map(|&c| u64::from(c)).sum::<u64>()).sum();
            let mean = sum as f64 / (597.0 * 448.0 * 3.0);
            assert!(mean > 20.0, "{}: the picture is lit (mean {mean:.1})", b.name);
        }
    }

    fn copy_tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap().flatten() {
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    /// Guest advanced and CRT-Royale from upstream, dropped in with
    /// upstream's folders (`crt/`, `include/`, `blurs/`) in a folder of the
    /// player's, are found by the catalogue under their file names, load
    /// and draw. A broken drop-in is listed too, and its load fails with an
    /// error naming it (the window then falls back to `OFF`). Skips without
    /// a graphics adapter.
    #[test]
    fn upstream_presets_dropped_in_load_and_draw_and_a_broken_one_fails() {
        let Some((device, queue, info)) = gpu() else { return };
        let dir = scratch("drop-ins");
        for folder in ["crt", "include", "blurs"] {
            copy_tree(&repo_shaders().join(folder), &dir.join("upstream").join(folder));
        }
        std::fs::write(dir.join("broken.slangp"), "shaders = 1\nshader0 = nowhere/gone.slang\n").unwrap();
        let catalogue = crate::shaders::Catalogue::discover(&repo_shaders(), &dir);
        for (id, name) in [("upstream/crt/crt-guest-advanced.slangp", "CRT-GUEST-ADVANCED"), ("upstream/crt/crt-royale.slangp", "CRT-ROYALE")] {
            let entry = catalogue.find(id).unwrap_or_else(|| panic!("{id} is listed"));
            assert_eq!(entry.name, name);
            let chain = load(entry.preset.as_ref().unwrap(), &device, &queue, &info).unwrap_or_else(|e| panic!("{id} loads: {e}"));
            let mut look = Look::new(id, chain, &device, wgpu::TextureFormat::Bgra8Unorm);
            let pixels = draw_offscreen(&device, &queue, &mut look, PictureRect { origin: (117, 26), size: (597, 448), scale: 2 });
            let lit = pixels[(250 * 832 + 400) as usize].iter().map(|&c| u32::from(c)).sum::<u32>();
            assert!(lit > 0, "{id} draws a picture");
            assert_eq!(pixels[(250 * 832 + 50) as usize], [0, 0, 0], "{id}: black beside the picture");
        }
        let broken = catalogue.find("broken.slangp").expect("a broken drop-in is listed");
        assert_eq!(broken.name, "BROKEN");
        let error = load(broken.preset.as_ref().unwrap(), &device, &queue, &info).err().expect("it fails to load");
        assert!(first_line(&error).starts_with("broken.slangp: "), "the notice names the drop-in by its file: {error}");
        assert!(error.contains("gone.slang"), "the error names the missing shader: {error}");
        assert!(error.ends_with(&dir.join("broken.slangp").display().to_string()), "the log gets the full path: {error}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A preset that names a missing shader, and one whose shader does not
    /// compile, each come back as an error naming the problem (through the
    /// loader's thread, as the window loads), never a crash. Skips without
    /// a graphics adapter.
    #[test]
    fn a_broken_preset_comes_back_as_an_error() {
        let Some((device, queue, info)) = gpu() else { return };
        let dir = scratch("broken");
        let missing = dir.join("missing.slangp");
        std::fs::write(&missing, "shaders = 1\nshader0 = nowhere/gone.slang\n").unwrap();
        let bad = dir.join("bad.slangp");
        std::fs::write(&bad, "shaders = 1\nshader0 = bad.slang\nscale_type0 = viewport\n").unwrap();
        std::fs::write(
            dir.join("bad.slang"),
            "#version 450\n#pragma stage vertex\nvoid main() { this is not glsl; }\n#pragma stage fragment\nvoid main() {}\n",
        )
        .unwrap();
        let mut loader = Loader::default();
        for (preset, mention) in [(missing, "gone.slang"), (bad.clone(), "bad"), (dir.join("absent.slangp"), "absent.slangp")] {
            loader.request(&LookRequest { id: "broken".into(), preset }, &device, &queue, &info);
            let (id, result, _) = loader.wait().expect("the loader answers");
            assert_eq!(id, "broken");
            let error = result.err().expect("a broken preset fails");
            assert!(error.contains(mention), "the error names {mention}: {error}");
            assert!(!first_line(&error).is_empty());
        }
        // A newer request supersedes an older one.
        loader.request(&LookRequest { id: "first".into(), preset: bad.clone() }, &device, &queue, &info);
        loader.request(&LookRequest { id: "second".into(), preset: bad }, &device, &queue, &info);
        assert_eq!(loader.wait().map(|(id, ..)| id).as_deref(), Some("second"));
        loader.cancel();
        assert!(loader.poll().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `ARCADE CABINET`'s parameters as the tuning screen gets them: Guest's
    /// well over a hundred, in the passes' own order with their headings,
    /// the preset's values as the look's own.
    #[test]
    fn arcade_cabinets_parameters_come_in_the_shaders_order_with_the_presets_values() {
        let params = parameters(&repo_shaders().join("looks/arcade-cabinet.slangp")).expect("the parameters read");
        let knobs = params.iter().filter(|p| !p.heading()).count();
        assert!(knobs > 100, "{knobs} knobs");
        let at = |id: &str| params.iter().position(|p| p.id == id).unwrap_or_else(|| panic!("{id} listed"));
        // afterglow0.slang (pass 2) declares its heading, then its knobs.
        assert!(params[at("bogus_afterglow")].heading());
        assert_eq!(at("esrc"), at("bogus_afterglow") + 1);
        assert!(at("PR") < at("PG") && at("PG") < at("PB"));
        assert!(at("bogus_afterglow") < at("bogus_rbloom"), "pass 2 before pass 4 (avg-lum)");
        let h_sharp = &params[at("h_sharp")];
        assert_eq!(h_sharp.initial, 6.0, "ARCADE CABINET's own value, not Guest's");
        assert!(h_sharp.min < h_sharp.max && h_sharp.step > 0.0);
        let mut ids: Vec<_> = params.iter().map(|p| &p.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), params.len(), "each parameter once");
    }

    /// A tuning reaches the running chain: the parameter's value changes,
    /// the next draw shows it, and an empty tuning puts the look's own
    /// values back. Skips without a graphics adapter.
    /// A device without the pipeline cache (wgpu has it on Vulkan, not on
    /// DX12) still loads a look: librashader's cache is asked for only where
    /// the device has it. Before, its cache made the pipeline cache whatever
    /// the device had, and the load panicked.
    #[test]
    fn a_look_loads_on_a_device_without_the_pipeline_cache() {
        let Some((device, queue, info)) = gpu_requesting(wanted_features().difference(wgpu::Features::PIPELINE_CACHE)) else { return };
        assert!(!device.features().contains(wgpu::Features::PIPELINE_CACHE));
        let preset = repo_shaders().join("looks/arcade-cabinet.slangp");
        load(&preset, &device, &queue, &info).expect("ARCADE CABINET loads without the pipeline cache");
    }

    #[test]
    fn a_tuning_is_set_on_the_running_chain_and_shows_on_the_next_draw() {
        let Some((device, queue, info)) = gpu() else { return };
        let preset = repo_shaders().join("looks/arcade-cabinet.slangp");
        let mut loader = Loader::default();
        loader.request(&LookRequest { id: "arcade_cabinet".into(), preset }, &device, &queue, &info);
        let (id, chain, _) = loader.wait().expect("the loader answers");
        let (pid, params) = loader.take_parameters().expect("the parameters come with the load");
        assert_eq!(pid, id);
        assert!(params.len() > 100);
        let mut look = Look::new(&id, chain.expect("ARCADE CABINET loads"), &device, wgpu::TextureFormat::Bgra8Unorm);
        look.set_defaults(&params);
        let rect = PictureRect { origin: (32, 26), size: (768, 448), scale: 2 };
        let mean = |pixels: &[[u8; 3]]| pixels.iter().flatten().map(|&c| f64::from(c)).sum::<f64>() / pixels.len() as f64 / 3.0;
        let plain = mean(&draw_offscreen(&device, &queue, &mut look, rect));
        assert_eq!(look.parameter("brightboost"), Some(1.55), "the preset's own value");
        look.tune(&[("brightboost".to_owned(), 4.0)]);
        assert_eq!(look.parameter("brightboost"), Some(4.0));
        let bright = mean(&draw_offscreen(&device, &queue, &mut look, rect));
        assert!(bright > plain + 5.0, "brighter at once: {plain:.1} -> {bright:.1}");
        look.tune(&[]);
        assert_eq!(look.parameter("brightboost"), Some(1.55), "back to the look's own");
        let again = mean(&draw_offscreen(&device, &queue, &mut look, rect));
        assert!((again - plain).abs() < 1.0, "as before: {plain:.1} and {again:.1}");
    }

    /// A look saved from `ARCADE CABINET` with `brightboost` at 4 (ticket
    /// #213), in a player's folder: its `#reference` is the built-in's id, so
    /// the parameters read from the shaders folder wherever the game is
    /// (here, and a copy of it elsewhere) are the saved look's, with the
    /// base's own values for everything it didn't set.
    #[test]
    fn a_saved_look_keeps_its_values_after_the_game_is_moved() {
        let app = scratch("saved-app");
        let moved = scratch("saved-moved");
        let drop_ins = crate::shaders::drop_in_dir(&app);
        let save = sf_engine::front::SaveLook {
            look: "arcade_cabinet".into(),
            name: "BRIGHT CAB".into(),
            overrides: vec![("brightboost".into(), 4.0)],
        };
        let id = crate::shaders::save_look(&drop_ins, &save).expect("the look saves");
        copy_tree(&repo_shaders(), &moved.join("game").join("shaders"));
        let base = parameters(&repo_shaders().join("looks/arcade-cabinet.slangp")).unwrap();
        let own = |params: &[LookParam], id: &str| params.iter().find(|p| p.id == id).map(|p| p.initial);
        for root in [repo_shaders(), moved.join("game").join("shaders")] {
            let preset = crate::shaders::loadable(&drop_ins.join(&id), &root).expect("it resolves");
            let params = parameters(&preset).unwrap_or_else(|e| panic!("reads from {}: {e}", root.display()));
            assert_eq!(own(&params, "brightboost"), Some(4.0), "the saved value, from {}", root.display());
            assert_eq!(own(&params, "h_sharp"), own(&base, "h_sharp"), "the base's own value for what it didn't set");
            assert_eq!(params.len(), base.len(), "the same parameters");
            crate::shaders::release(&preset);
            assert!(!preset.parent().unwrap().exists(), "its own temporary folder, released");
        }
        for d in [app, moved] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    /// A saved look is listed, loads through the loader as any look does and
    /// draws with its values: brighter than the look it was saved from.
    /// Skips without a graphics adapter.
    #[test]
    fn a_saved_look_loads_and_draws_with_its_values() {
        let Some((device, queue, info)) = gpu() else { return };
        let app = scratch("saved-draw");
        let drop_ins = crate::shaders::drop_in_dir(&app);
        let save = sf_engine::front::SaveLook {
            look: "arcade_cabinet".into(),
            name: "BRIGHT CAB".into(),
            overrides: vec![("brightboost".into(), 4.0)],
        };
        let id = crate::shaders::save_look(&drop_ins, &save).expect("the look saves");
        let catalogue = crate::shaders::Catalogue::discover(&repo_shaders(), &drop_ins);
        assert_eq!(catalogue.saved(), [id.as_str()]);
        let rect = PictureRect { origin: (32, 26), size: (768, 448), scale: 2 };
        let mean = |pixels: &[[u8; 3]]| pixels.iter().flatten().map(|&c| f64::from(c)).sum::<f64>() / pixels.len() as f64 / 3.0;
        let mut loader = Loader::default();
        let mut drawn = Vec::new();
        for look in ["arcade_cabinet", id.as_str()] {
            let entry = catalogue.find(look).unwrap_or_else(|| panic!("{look} is listed"));
            let preset = crate::shaders::loadable(entry.preset.as_ref().unwrap(), &repo_shaders()).unwrap();
            let temporary = preset != *entry.preset.as_ref().unwrap();
            loader.request(&LookRequest { id: look.into(), preset: preset.clone() }, &device, &queue, &info);
            let (got, chain, _) = loader.wait().expect("the loader answers");
            // The saved look's temporary preset is gone once loaded, and the
            // chain draws without it; a built-in's own preset stays.
            assert_eq!(preset.exists(), !temporary, "{look}: {}", preset.display());
            let (_, params) = loader.take_parameters().expect("the parameters come with the load");
            let mut look = Look::new(&got, chain.unwrap_or_else(|e| panic!("{got} loads: {e}")), &device, wgpu::TextureFormat::Bgra8Unorm);
            look.set_defaults(&params);
            drawn.push((look.parameter("brightboost"), mean(&draw_offscreen(&device, &queue, &mut look, rect))));
        }
        assert_eq!(drawn[0].0, Some(1.55), "the base's own value");
        assert_eq!(drawn[1].0, Some(4.0), "the saved look's");
        assert!(drawn[1].1 > drawn[0].1 + 5.0, "drawn with it: {:.1} -> {:.1}", drawn[0].1, drawn[1].1);
        let _ = std::fs::remove_dir_all(&app);
    }

    /// A measurement, not a check: each built-in look (and each preset in
    /// `SF_LOOKS`, `;`-separated) drawn offscreen at true 4:3 on the
    /// user's 3200x2000 screen (a 2389x1792 picture). Per game frame
    /// (upload, chain, composite) and per plain present (the composite
    /// alone): GPU time from timestamps where the adapter has them, and
    /// wall time waited on, as median / p95 / max ms. The looks take turns
    /// in rounds (`SF_ROUNDS`, 6 by default), so a GPU busy with other
    /// programs weighs on all of them alike. Run it with
    /// `cargo test -p sf-host --release time_looks -- --ignored --nocapture`.
    #[test]
    #[ignore = "a GPU measurement, run by hand"]
    fn time_looks() {
        let stamps = wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        let Some((device, queue, info)) = gpu_with(stamps) else { return };
        let stamps = device.features().contains(stamps);
        eprintln!("adapter: {} ({:?}), timestamps: {stamps}", info.name, info.backend);
        let mut presets: Vec<(String, PathBuf)> = crate::shaders::BUILT_IN
            .iter()
            .map(|b| (b.name.to_owned(), repo_shaders().join(b.preset)))
            .collect();
        if let Ok(more) = std::env::var("SF_LOOKS") {
            presets.extend(more.split(';').filter(|p| !p.is_empty()).map(|p| (p.to_owned(), PathBuf::from(p))));
        }
        let rounds: usize = std::env::var("SF_ROUNDS").ok().and_then(|r| r.parse().ok()).unwrap_or(6);
        let (w, h) = (3200u32, 2000u32);
        let rect = PictureRect { origin: ((w as i32 - 2389) / 2, (h as i32 - 1792) / 2), size: (2389, 1792), scale: 8 };
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("target"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let queries = stamps.then(|| {
            let set = device.create_query_set(&wgpu::QuerySetDescriptor { label: None, ty: wgpu::QueryType::Timestamp, count: 2 });
            let resolve = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 16,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
            let read = device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: 16,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            (set, resolve, read)
        });
        let period = f64::from(queue.get_timestamp_period());
        let ms = |v: &[f64]| {
            let mut v = v.to_vec();
            v.sort_by(f64::total_cmp);
            let at = |q: f64| v[((v.len() - 1) as f64 * q).round() as usize];
            format!("{:.2} / {:.2} / {:.2}", at(0.5), at(0.95), v[v.len() - 1])
        };
        let mut looks = Vec::new();
        for (name, preset) in presets {
            let start = Instant::now();
            match load(&preset, &device, &queue, &info) {
                Ok(chain) => {
                    let took = start.elapsed().as_secs_f64();
                    looks.push((name.clone(), took, Look::new(&name, chain, &device, wgpu::TextureFormat::Bgra8UnormSrgb)));
                }
                Err(e) => eprintln!("{name}: {}", first_line(&e)),
            }
        }
        let mut frame = Frame { argb: vec![0xff20_2040; WIDTH * HEIGHT] };
        let mut tick = 0usize;
        // One draw, a new game frame or a plain present: (wall, GPU) ms.
        let mut draw = |look: &mut Look, fresh: bool| -> (f64, f64) {
            let t = Instant::now();
            if fresh {
                // A bar sweeping across, as the smoke check's picture.
                tick += 1;
                for (p, px) in frame.argb.iter_mut().enumerate() {
                    *px = if p % WIDTH == tick % WIDTH { 0xffff_ffff } else { 0xff20_2040 + (p / WIDTH) as u32 };
                }
                look.upload(&queue, &frame);
            }
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            if let Some((set, ..)) = &queries {
                encoder.write_timestamp(set, 0);
            }
            look.draw(&device, &queue, &mut encoder, &view, rect).expect("the look draws");
            if let Some((set, resolve, read)) = &queries {
                encoder.write_timestamp(set, 1);
                encoder.resolve_query_set(set, 0..2, resolve, 0);
                encoder.copy_buffer_to_buffer(resolve, 0, read, 0, 16);
            }
            queue.submit([encoder.finish()]);
            device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).unwrap();
            let wall = t.elapsed().as_secs_f64() * 1000.0;
            let gpu = queries.as_ref().map_or(f64::NAN, |(_, _, read)| {
                read.slice(..).map_async(wgpu::MapMode::Read, |r| r.expect("map the timestamps"));
                device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).unwrap();
                let bytes = read.slice(..).get_mapped_range().expect("the timestamps").to_vec();
                read.unmap();
                let at = |i: usize| u64::from_le_bytes(bytes[i * 8..i * 8 + 8].try_into().unwrap());
                at(1).saturating_sub(at(0)) as f64 * period / 1e6
            });
            (wall, gpu)
        };
        // Per look: its game frames and its plain presents.
        let mut times: Vec<[Vec<(f64, f64)>; 2]> = looks.iter().map(|_| [Vec::new(), Vec::new()]).collect();
        for (_, _, look) in &mut looks {
            for _ in 0..20 {
                draw(look, true);
            }
        }
        for _ in 0..rounds {
            for ((_, _, look), t) in looks.iter_mut().zip(&mut times) {
                for _ in 0..50 {
                    t[0].push(draw(look, true));
                }
                for _ in 0..50 {
                    t[1].push(draw(look, false));
                }
            }
        }
        for ((name, took, _), [frames, presents]) in looks.iter().zip(&times) {
            let gpu = |v: &[(f64, f64)]| ms(&v.iter().map(|t| t.1).collect::<Vec<_>>());
            let wall = |v: &[(f64, f64)]| ms(&v.iter().map(|t| t.0).collect::<Vec<_>>());
            eprintln!(
                "{name}: load {took:.2} s\n  game frame    GPU {} ms, wall {} ms\n  plain present GPU {} ms, wall {} ms",
                gpu(frames),
                wall(frames),
                gpu(presents),
                wall(presents)
            );
        }
    }

    #[test]
    fn the_first_line_of_an_error_is_its_first_non_empty_line() {
        assert_eq!(first_line("\n  a.slangp: shader compile error\n  at line 3\n"), "a.slangp: shader compile error");
        assert_eq!(first_line("one"), "one");
    }
}