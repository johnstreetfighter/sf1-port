//! The engine's panels on the GPU (ticket #212, VO9, the tuning screen's;
//! ticket #227, ML1, the move list's; design section 8): the engine's
//! panel layer drawn sharp over the look, after the filter chain, at a
//! whole-number scale of the ROM font, at its side of the surface (the
//! tuning screen's at the right, the move list's at the left). While it
//! shows, the picture moves to the other edge ([`beside_panel`]), so the
//! panel covers the black margin and a strip of the picture rather than
//! half of it.

use sf_engine::front::tune::{PANEL_COLS, PANEL_ROWS};
use sf_engine::front::{Panel, PanelSide};

use crate::picture::PictureRect;

/// The tuning panel's size in pixels at 1×, which a layer starts at.
pub const PANEL_SIZE: (u32, u32) = (PANEL_COLS as u32 * 8, PANEL_ROWS as u32 * 8);

/// A native game frame fits either panel and leaves a strip of the fight.
pub const MIN_SURFACE_SIZE: (u32, u32) = (
    sf_video::render::WIDTH as u32,
    sf_video::render::HEIGHT as u32,
);

/// Where a panel of `size` pixels goes on a surface: its top-left corner
/// and its whole-number scale (at least 1). MOVES uses up to three quarters
/// of the height and five sixths of the width, keeping its text readable
/// even in a narrow 4:3 window while leaving a strip of the paused fight.
/// Tuning uses the full height and two fifths of the width. A panel sits at
/// its edge of the surface, centred down.
pub fn panel_place(surface: (u32, u32), size: (u32, u32), side: PanelSide) -> ((i32, i32), u32) {
    let (w, h) = size;
    let (width, height) = match side {
        PanelSide::Left => ((u64::from(surface.0) * 5 / 6) as u32, (u64::from(surface.1) * 3 / 4) as u32),
        PanelSide::Right => ((u64::from(surface.0) * 2 / 5) as u32, surface.1),
    };
    let scale = (height / h).min(width / w).max(1);
    let x = match side {
        PanelSide::Right => i64::from(surface.0) - i64::from(w * scale),
        PanelSide::Left => 0,
    };
    let y = (i64::from(surface.1) - i64::from(h * scale)) / 2;
    ((x as i32, y as i32), scale)
}

/// The picture moved to the edge of a surface `surface_width` wide that
/// the panel at `side` leaves free, for while the panel shows: the left
/// edge for a panel at the right, the right edge for one at the left. A
/// picture already overhanging keeps its place.
pub fn beside_panel(rect: PictureRect, surface_width: u32, side: PanelSide) -> PictureRect {
    let x = match side {
        PanelSide::Right => rect.origin.0.min(0),
        PanelSide::Left if rect.origin.0 < 0 => rect.origin.0,
        PanelSide::Left => rect.origin.0.max(surface_width as i32 - rect.size.0 as i32),
    };
    PictureRect {
        origin: (x, rect.origin.1),
        ..rect
    }
}

/// Each surface pixel inside the panel shows the panel pixel under it at
/// the whole-number scale; outside, the pass leaves the target alone.
const SHADER: &str = r#"
struct Place { origin: vec2<f32>, scale: f32, pad: f32 };
@group(0) @binding(0) var panel: texture_2d<f32>;
@group(0) @binding(1) var<uniform> place: Place;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(i & 1u) * 4 - 1);
    let y = f32(i32(i & 2u) * 2 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let offset = vec2<i32>(floor(at.xy)) - vec2<i32>(place.origin);
    if (any(offset < vec2<i32>(0))) {
        discard;
    }
    // Integer division keeps every source pixel exactly scale pixels wide,
    // including scales such as 3x whose GPU reciprocal can round down.
    let p = offset / vec2<i32>(i32(place.scale));
    let size = vec2<i32>(textureDimensions(panel));
    if (any(p >= size)) {
        discard;
    }
    return vec4<f32>(textureLoad(panel, p, 0).rgb, 1.0);
}
"#;

/// The panel's texture and the pass that draws it onto targets of one
/// format.
pub struct PanelLayer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    texture: wgpu::Texture,
    place: wgpu::Buffer,
    bytes: Vec<u8>,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    side: PanelSide,
}

/// A panel's texture, `size` pixels, in the format a target of `target`'s
/// takes: on an sRGB target the texture is sRGB too, so the two
/// conversions cancel and the ROM's colours go out as they are.
fn panel_texture(device: &wgpu::Device, target: wgpu::TextureFormat, size: (u32, u32)) -> (wgpu::Texture, wgpu::TextureFormat) {
    let format = if target.is_srgb() { wgpu::TextureFormat::Bgra8UnormSrgb } else { wgpu::TextureFormat::Bgra8Unorm };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("panel"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    (texture, format)
}

fn panel_bind_group(device: &wgpu::Device, pipeline: &wgpu::RenderPipeline, texture: &wgpu::Texture, place: &wgpu::Buffer) -> wgpu::BindGroup {
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("panel"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: place.as_entire_binding(),
            },
        ],
    })
}

impl PanelLayer {
    pub fn new(device: &wgpu::Device, target: wgpu::TextureFormat) -> PanelLayer {
        let (texture, format) = panel_texture(device, target, PANEL_SIZE);
        let place = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("panel place"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("panel"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("panel"),
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
        let bind_group = panel_bind_group(device, &pipeline, &texture, &place);
        PanelLayer {
            pipeline,
            bind_group,
            texture,
            place,
            bytes: vec![0; (PANEL_SIZE.0 * PANEL_SIZE.1 * 4) as usize],
            format,
            size: PANEL_SIZE,
            side: PanelSide::Right,
        }
    }

    /// The edge the uploaded panel sits at.
    pub fn side(&self) -> PanelSide {
        self.side
    }

    /// Copy the engine's panel (ARGB words, so B, G, R, A in memory) to the
    /// GPU, which takes its size and side. A panel whose pixels do not fill
    /// its size is ignored. After a panel of a new size or side the caller
    /// places it again ([`Self::place`]).
    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, panel: &Panel) {
        let size = (panel.width as u32, panel.height as u32);
        if size.0 == 0 || size.1 == 0 || panel.argb.len() != panel.width * panel.height {
            return;
        }
        if size != self.size {
            let (texture, _) = panel_texture(device, self.format, size);
            self.bind_group = panel_bind_group(device, &self.pipeline, &texture, &self.place);
            self.texture = texture;
            self.bytes = vec![0; (size.0 * size.1 * 4) as usize];
            self.size = size;
        }
        self.side = panel.side;
        for (out, px) in self.bytes.chunks_exact_mut(4).zip(&panel.argb) {
            out.copy_from_slice(&px.to_le_bytes());
        }
        queue.write_texture(
            self.texture.as_image_copy(),
            &self.bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.size.0 * 4),
                rows_per_image: Some(self.size.1),
            },
            self.texture.size(),
        );
    }

    /// The uploaded panel's size in pixels at 1×.
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// Place the panel for a surface of `surface` pixels ([`panel_place`]).
    pub fn place(&self, queue: &wgpu::Queue, surface: (u32, u32)) {
        let ((x, y), scale) = panel_place(surface, self.size, self.side);
        let values = [x as f32, y as f32, scale as f32, 0.0];
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        queue.write_buffer(&self.place, 0, &bytes);
    }

    /// Draw the panel over what `view` already holds.
    pub fn draw(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("panel"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::picture::picture_rect;
    use sf_engine::front::moves::{MOVES_COLS, MOVES_ROWS};
    use sf_engine::front::settings::Aspect;

    #[test]
    fn moves_are_readable_when_a_window_grows_to_normal_play_sizes() {
        let size = ((MOVES_COLS * 8) as u32, (MOVES_ROWS * 8) as u32);
        for (surface, minimum_scale) in [(MIN_SURFACE_SIZE, 1), ((768, 672), 2), ((896, 672), 2), ((1920, 1080), 3), ((3200, 2000), 6)] {
            let ((x, y), scale) = panel_place(surface, size, PanelSide::Left);
            assert!(scale >= minimum_scale, "MOVES text is only {scale}x at {surface:?}");
            assert!(x >= 0 && y >= 0);
            assert!(x as u32 + size.0 * scale <= surface.0);
            assert!(y as u32 + size.1 * scale <= surface.1);
            assert!(surface.0 - size.0 * scale >= surface.0 / 6, "the paused fight stays visible");
        }
    }

    #[test]
    fn resizing_keeps_every_panel_pixel_inside_a_supported_window() {
        let size = ((MOVES_COLS * 8) as u32, (MOVES_ROWS * 8) as u32);
        for width in (MIN_SURFACE_SIZE.0..=3200).step_by(37) {
            for height in (MIN_SURFACE_SIZE.1..=2000).step_by(29) {
                let ((x, y), scale) = panel_place((width, height), size, PanelSide::Left);
                assert!(x >= 0 && y >= 0);
                assert!(x as u32 + size.0 * scale <= width);
                assert!(y as u32 + size.1 * scale <= height);
                assert!(width - size.0 * scale >= width / 6, "the paused fight stays visible");
            }
        }
    }

    #[test]
    fn the_panel_takes_the_right_edge_at_a_whole_scale_and_the_picture_moves_left() {
        // 3200x2000 at true 4:3: the panel at 5x (1280x1120) covers the
        // 811-pixel margin and a 469-pixel strip of the 2389-pixel picture.
        let right = |surface| panel_place(surface, PANEL_SIZE, PanelSide::Right);
        assert_eq!(right((3200, 2000)), ((1920, 440), 5));
        let rect = beside_panel(picture_rect((3200, 2000), Aspect::True4x3), 3200, PanelSide::Right);
        assert_eq!((rect.origin, rect.size), ((0, 104), (2389, 1792)));
        assert_eq!(right((1920, 1080)), ((1152, 204), 3));
        assert_eq!(beside_panel(picture_rect((1920, 1080), Aspect::True4x3), 1920, PanelSide::Right).origin, (0, 92));
        // A 3x window is its own picture: the panel goes over it at 1x.
        assert_eq!(right((896, 672)), ((640, 224), 1));
        // Too small for even 1x: still 1x, hanging over the edges.
        assert_eq!(right((200, 100)).1, 1);
        // A picture already overhanging keeps its place.
        let wide = PictureRect { origin: (-42, -12), size: (384, 224), scale: 1 };
        assert_eq!(beside_panel(wide, 300, PanelSide::Right), wide);
        assert_eq!(beside_panel(wide, 300, PanelSide::Left), wide);
    }

    #[test]
    fn the_move_list_takes_the_left_edge_and_the_picture_moves_right() {
        // The move list is 40 x 28 cells (320 x 224): at 3200x2000 it is
        // 6x, with space for the paused fight on its right.
        let size = (320, 224);
        assert_eq!(panel_place((3200, 2000), size, PanelSide::Left), ((0, 328), 6));
        // The 2389-wide picture moves from x = 405 to the right edge.
        let rect = beside_panel(picture_rect((3200, 2000), Aspect::True4x3), 3200, PanelSide::Left);
        assert_eq!((rect.origin, rect.size), ((811, 104), (2389, 1792)));
        // A 3x window is its own picture: MOVES uses 2x text on its left.
        assert_eq!(panel_place((896, 672), size, PanelSide::Left), ((0, 112), 2));
        assert_eq!(beside_panel(picture_rect((896, 672), Aspect::True4x3), 896, PanelSide::Left).origin.0, 0);
    }

    /// A non-power-of-two scale must repeat every source pixel exactly:
    /// float division on the GPU can round a pixel boundary down by one.
    #[test]
    fn moves_pixels_repeat_exactly_at_three_and_six_times_scale() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            eprintln!("skipping: no graphics adapter");
            return;
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).expect("a device");
        let size = ((MOVES_COLS * 8) as u32, (MOVES_ROWS * 8) as u32);
        let mut seed = 99u32;
        let panel = Panel {
            side: PanelSide::Left,
            width: size.0 as usize,
            height: size.1 as usize,
            argb: (0..size.0 * size.1)
                .map(|_| {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    0xff00_0000 | (seed >> 8)
                })
                .collect(),
        };
        let format = wgpu::TextureFormat::Bgra8Unorm;
        let mut layer = PanelLayer::new(&device, format);
        layer.upload(&device, &queue, &panel);
        for ((w, h), expected_scale) in [((1920u32, 1080u32), 3), ((3200, 2000), 6)] {
            let ((px, py), scale) = panel_place((w, h), size, panel.side);
            assert_eq!(scale, expected_scale);
            layer.place(&queue, (w, h));
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("moves exact scaling"),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let readback = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("moves exact scaling readback"),
                size: u64::from(w * h * 4),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let view = target.create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear outside moves"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            }));
            layer.draw(&mut encoder, &view);
            encoder.copy_texture_to_buffer(
                target.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &readback,
                    layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
                },
                target.size(),
            );
            queue.submit([encoder.finish()]);
            readback.slice(..).map_async(wgpu::MapMode::Read, |r| r.expect("map readback"));
            device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).unwrap();
            let bytes = readback.slice(..).get_mapped_range().expect("mapped readback");
            for y in 0..h {
                for x in 0..w {
                    let (dx, dy) = (x as i32 - px, y as i32 - py);
                    let expected = if dx >= 0 && dy >= 0 && dx < (size.0 * scale) as i32 && dy < (size.1 * scale) as i32 {
                        panel.argb[((dy as u32 / scale) * size.0 + dx as u32 / scale) as usize]
                    } else {
                        0xff00_0000
                    };
                    let at = ((y * w + x) * 4) as usize;
                    let actual = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
                    assert_eq!(actual, expected, "pixel ({x},{y}) at {scale}x");
                }
            }
        }
    }

    /// On the real GPU, offscreen: `ARCADE CABINET` drawn into the picture
    /// moved to the left edge, then the panel over it. Every panel pixel
    /// is the panel's own, untouched by the look (sharp), the picture starts
    /// at the left edge, and the rest is black. Skips without an adapter.
    #[test]
    fn the_panel_is_drawn_sharp_over_the_look_with_the_picture_at_the_left() {
        use crate::look::{load, wanted_features, Look};
        use sf_video::render::{Frame, HEIGHT, WIDTH};
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN | wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
            eprintln!("skipping: no graphics adapter");
            return;
        };
        let mut limits = wgpu::Limits::default();
        limits.max_inter_stage_shader_variables = adapter.limits().max_inter_stage_shader_variables;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("panel test"),
            required_features: wanted_features() & adapter.features(),
            required_limits: limits,
            ..wgpu::DeviceDescriptor::default()
        }))
        .expect("a device");
        let preset = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../shaders/looks/arcade-cabinet.slangp");
        let chain = load(&preset, &device, &queue, &adapter.get_info()).expect("ARCADE CABINET loads");
        let format = wgpu::TextureFormat::Bgra8Unorm;
        let mut look = Look::new("arcade_cabinet", chain, &device, format);
        look.upload(&queue, &Frame { argb: vec![0xffc0_8040; WIDTH * HEIGHT] });
        let mut seed = 99u32;
        let panel = Panel {
            side: PanelSide::Right,
            width: PANEL_SIZE.0 as usize,
            height: PANEL_SIZE.1 as usize,
            argb: (0..PANEL_SIZE.0 * PANEL_SIZE.1)
                .map(|_| {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    0xff00_0000 | (seed >> 8)
                })
                .collect(),
        };
        let mut layer = PanelLayer::new(&device, format);
        layer.upload(&device, &queue, &panel);
        // 832x500: the picture 2x tall at true 4:3 (597x448), moved from
        // x = 117 to the left edge; the panel at 1x at the right.
        let (w, h) = (832u32, 500u32);
        layer.place(&queue, (w, h));
        let rect = beside_panel(picture_rect((w, h), Aspect::True4x3), w, PanelSide::Right);
        assert_eq!((rect.origin, rect.size), ((0, 26), (597, 448)));
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("target"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
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
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        look.draw(&device, &queue, &mut encoder, &view, rect).expect("the look draws");
        layer.draw(&mut encoder, &view);
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
            },
            target.size(),
        );
        queue.submit([encoder.finish()]);
        readback.slice(..).map_async(wgpu::MapMode::Read, |r| r.expect("map the readback"));
        device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None }).unwrap();
        let bytes = readback.slice(..).get_mapped_range().expect("the mapped readback").to_vec();
        let at = |x: u32, y: u32| {
            let i = ((y * w + x) * 4) as usize;
            u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], 0xff])
        };
        let ((px, py), scale) = panel_place((w, h), PANEL_SIZE, PanelSide::Right);
        assert_eq!(((px, py), scale), ((576, 138), 1));
        let (mut wrong, mut lit_left, mut stray) = (0, 0, 0);
        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = (x as i32 - px, y as i32 - py);
                let in_panel = dx >= 0 && dy >= 0 && (dx as u32) < PANEL_SIZE.0 && (dy as u32) < PANEL_SIZE.1;
                let in_picture = x < 597 && (26..26 + 448).contains(&y);
                if in_panel {
                    wrong += usize::from(at(x, y) != panel.argb[(dy as u32 * PANEL_SIZE.0 + dx as u32) as usize]);
                } else if in_picture {
                    lit_left += usize::from(x < 117 && at(x, y) != 0xff00_0000);
                } else {
                    stray += usize::from(at(x, y) != 0xff00_0000);
                }
            }
        }
        assert_eq!(wrong, 0, "every panel pixel is the panel's own");
        assert!(lit_left > 117 * 448 / 2, "the picture starts at the left edge ({lit_left} lit left of x = 117)");
        assert_eq!(stray, 0, "black outside the picture and the panel");
    }
}
