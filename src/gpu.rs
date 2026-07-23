//! wgpu presentation: the 320x200 palettized game frame as a nearest-scaled quad, plus
//! the 3D assets (the manu3 hand) rendered as REAL GPU triangles at window resolution —
//! the same decoded game math (skeletal compose + cursor-centred projection) supplies
//! the vertices; the GPU rasterizes them with per-pixel texel sampling (screen-affine
//! interpolation, matching the game's affine fill) and a depth buffer. This is what
//! gives the hand crisp high-resolution edges instead of 320x200 software texels.

use std::num::NonZeroU32;
use std::ptr::NonNull;

/// One textured triangle of a 3D asset in 320x200 virtual-screen space.
/// Vertex layout: x, y (virtual screen), depth (game units), u, v (texel coords).
#[derive(Clone, Copy)]
pub struct HandTri(pub [[f32; 5]; 3]);

pub struct GpuPresenter {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    bg_pipeline: wgpu::RenderPipeline,
    hand_pipeline: wgpu::RenderPipeline,
    bg_tex: wgpu::Texture,
    bg_bind: wgpu::BindGroup,
    hand_bind: wgpu::BindGroup,
    pal_tex: wgpu::Texture,
    quad_vbuf: wgpu::Buffer,
    hand_vbuf: wgpu::Buffer,
    hand_vcap: usize,
    star_pipeline: wgpu::RenderPipeline,
    star_bind: wgpu::BindGroup,
    star_vbuf: wgpu::Buffer,
    star_vcap: usize,
    depth: wgpu::TextureView,
}

const BG_SHADER: &str = r#"
struct VOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex
fn vs(@location(0) p: vec2<f32>, @location(1) uv: vec2<f32>) -> VOut {
    var o: VOut;
    o.pos = vec4<f32>(p, 0.0, 1.0);
    o.uv = uv;
    return o;
}
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@fragment
fn fs(in: VOut) -> @location(0) vec4<f32> {
    return textureSample(tex, samp, in.uv);
}
"#;

const STAR_SHADER: &str = r#"
struct VOut { @builtin(position) pos: vec4<f32>, @location(0) @interpolate(flat) shade: u32 };
@vertex
fn vs(@location(0) p: vec2<f32>, @location(1) shade: u32) -> VOut {
    var o: VOut;
    o.pos = vec4<f32>(p, 0.999, 1.0);
    o.shade = shade;
    return o;
}
@group(0) @binding(0) var pal: texture_2d<f32>;
@fragment
fn fs(in: VOut) -> @location(0) vec4<f32> {
    return textureLoad(pal, vec2<i32>(i32(in.shade), 0), 0);
}
"#;

const HAND_SHADER: &str = r#"
struct VOut {
    @builtin(position) pos: vec4<f32>,
    // Screen-affine interpolation (linear, no perspective) — the game's affine fill.
    @location(0) @interpolate(linear) uv: vec2<f32>,
};
@vertex
fn vs(@location(0) p: vec3<f32>, @location(1) uv: vec2<f32>) -> VOut {
    var o: VOut;
    o.pos = vec4<f32>(p.xy, p.z, 1.0);
    o.uv = uv;
    return o;
}
@group(0) @binding(0) var tex: texture_2d<u32>;
@group(0) @binding(1) var pal: texture_2d<f32>;
@fragment
fn fs(in: VOut) -> @location(0) vec4<f32> {
    let dims = textureDimensions(tex);
    let tx = clamp(i32(in.uv.x), 0, i32(dims.x) - 1);
    // The seg4 texture's material spans rows 0..62 — the mesh's whole v range
    // (fs:[4]=1C94 capture); clamp only as an interpolation-overshoot bound.
    let ty = clamp(i32(in.uv.y), 0, 62);
    // The game's affine fill writes EVERY texel unconditionally (0xC2A: mov es:[di],ch
    // with no zero test) — texel 0 is opaque palette black (the wrist's dither), not
    // transparency.
    let texel = textureLoad(tex, vec2<i32>(tx, ty), 0).r;
    return textureLoad(pal, vec2<i32>(i32(texel), 0), 0);
}
"#;

impl GpuPresenter {
    /// Create over an existing X11 (xcb) window.
    ///
    /// # Safety
    /// `xcb_conn` must be a live xcb_connection_t pointer outliving the presenter.
    pub unsafe fn new(
        xcb_conn: *mut std::ffi::c_void,
        screen: i32,
        window: u32,
        win_w: u32,
        win_h: u32,
        hand_tex: &[u8],
        hand_tex_w: u32,
    ) -> anyhow::Result<GpuPresenter> {
        use raw_window_handle::{
            RawDisplayHandle, RawWindowHandle, XcbDisplayHandle, XcbWindowHandle,
        };
        // Try Vulkan first, then GL (EGL-X11) — headless/nix setups often present
        // only via one of them. A failed Surface::configure is detected with an
        // error scope instead of the fatal default handler.
        let mut picked = None;
        for backend in [wgpu::Backends::VULKAN, wgpu::Backends::GL] {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: backend,
                ..Default::default()
            });
            let dh =
                RawDisplayHandle::Xcb(XcbDisplayHandle::new(NonNull::new(xcb_conn), screen));
            let wh = RawWindowHandle::Xcb(XcbWindowHandle::new(
                NonZeroU32::new(window).ok_or_else(|| anyhow::anyhow!("zero window id"))?,
            ));
            let surface = match unsafe {
                instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: dh,
                    raw_window_handle: wh,
                })
            } {
                Ok(sf) => sf,
                Err(e) => {
                    eprintln!("[gpu] {backend:?}: surface creation failed: {e}");
                    continue;
                }
            };
            let Ok(adapter) =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::LowPower,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                }))
            else {
                eprintln!("[gpu] {backend:?}: no adapter");
                continue;
            };
            let Ok((device, queue)) = pollster::block_on(
                adapter.request_device(&wgpu::DeviceDescriptor::default()),
            ) else {
                eprintln!("[gpu] {backend:?}: no device");
                continue;
            };
            let caps = surface.get_capabilities(&adapter);
            if caps.formats.is_empty() {
                eprintln!("[gpu] {backend:?}: no surface formats");
                continue;
            }
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|f| !f.is_srgb())
                .unwrap_or(caps.formats[0]);
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: win_w.max(1),
                height: win_h.max(1),
                present_mode: wgpu::PresentMode::AutoVsync,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };
            device.push_error_scope(wgpu::ErrorFilter::Validation);
            surface.configure(&device, &config);
            if let Some(e) = pollster::block_on(device.pop_error_scope()) {
                eprintln!(
                    "[gpu] {backend:?}: configure rejected: {e:?}\n  formats {:?} alpha {:?} modes {:?}",
                    caps.formats, caps.alpha_modes, caps.present_modes
                );
                continue;
            }
            eprintln!("[gpu] backend: {:?}", adapter.get_info().backend);
            picked = Some((surface, device, queue, config, format));
            break;
        }
        let Some((surface, device, queue, config, format)) = picked else {
            anyhow::bail!("no presentable wgpu backend");
        };

        // Background: 320x200 RGBA texture + nearest sampler.
        let bg_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bg"),
            size: wgpu::Extent3d { width: 320, height: 200, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let bg_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bg"),
            source: wgpu::ShaderSource::Wgsl(BG_SHADER.into()),
        });
        let bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bg_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &bg_tex.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let bg_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bg_layout],
            push_constant_ranges: &[],
        });
        let bg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bg"),
            layout: Some(&bg_pl),
            vertex: wgpu::VertexState {
                module: &bg_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 16,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &bg_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            // Shares the pass's depth attachment; never tests or writes it.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        // Hand: R8Uint game texture + palette LUT + depth buffer.
        let tex_h = (hand_tex.len() as u32).div_ceil(hand_tex_w);
        let hand_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hand-tex"),
            size: wgpu::Extent3d {
                width: hand_tex_w,
                height: tex_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut padded = vec![0u8; (hand_tex_w * tex_h) as usize];
        padded[..hand_tex.len()].copy_from_slice(hand_tex);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &hand_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &padded,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(hand_tex_w),
                rows_per_image: Some(tex_h),
            },
            wgpu::Extent3d { width: hand_tex_w, height: tex_h, depth_or_array_layers: 1 },
        );
        let pal_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("palette"),
            size: wgpu::Extent3d { width: 256, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let hand_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hand"),
            source: wgpu::ShaderSource::Wgsl(HAND_SHADER.into()),
        });
        let hand_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Uint,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                    count: None,
                },
            ],
        });
        let hand_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &hand_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &hand_texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &pal_tex.create_view(&Default::default()),
                    ),
                },
            ],
        });
        let hand_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&hand_layout],
            push_constant_ranges: &[],
        });
        let hand_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hand"),
            layout: Some(&hand_pl),
            vertex: wgpu::VertexState {
                module: &hand_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 20,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &hand_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        // Starfield: 1-game-pixel quads at subpixel positions, palette-shaded, drawn
        // under the (colour-keyed) panorama.
        let star_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("stars"),
            source: wgpu::ShaderSource::Wgsl(STAR_SHADER.into()),
        });
        let star_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                },
                count: None,
            }],
        });
        let star_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &star_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(
                    &pal_tex.create_view(&Default::default()),
                ),
            }],
        });
        let star_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&star_layout],
            push_constant_ranges: &[],
        });
        let star_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("stars"),
            layout: Some(&star_pl),
            vertex: wgpu::VertexState {
                module: &star_shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 12,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Uint32],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &star_shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(format.into())],
            }),
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let star_vcap = 4096 * 6;
        let star_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("stars"),
            size: (star_vcap * 12) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let quad_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad"),
            size: 6 * 16,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let hand_vcap = 4096;
        let hand_vbuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hand-verts"),
            size: (hand_vcap * 20) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let depth = Self::make_depth(&device, config.width, config.height);
        Ok(GpuPresenter {
            surface,
            device,
            queue,
            config,
            bg_pipeline,
            hand_pipeline,
            bg_tex,
            bg_bind,
            hand_bind,
            pal_tex,
            quad_vbuf,
            hand_vbuf,
            hand_vcap,
            star_pipeline,
            star_bind,
            star_vbuf,
            star_vcap,
            depth,
        })
    }

    fn make_depth(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default())
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.config.width = w.max(1);
        self.config.height = h.max(1);
        self.surface.configure(&self.device, &self.config);
        self.depth = Self::make_depth(&self.device, self.config.width, self.config.height);
    }

    /// Present one frame: the palettized 320x200 background integer-scaled with
    /// letterboxing, then the hand triangles at full window resolution.
    pub fn present(
        &mut self,
        indices: &[u8],
        palette: &[[u8; 3]; 256],
        tris: &[HandTri],
        stars: &[(u16, u16, u8)],
        colorkey: bool,
    ) -> anyhow::Result<()> {
        // Background texel upload; with a colour key, index 0 becomes transparent
        // (the bridge windows) so the GPU stars show through from behind.
        let mut rgba = vec![0u8; 320 * 200 * 4];
        for (i, &p) in indices.iter().take(320 * 200).enumerate() {
            let c = palette[p as usize];
            rgba[i * 4..i * 4 + 3].copy_from_slice(&c);
            rgba[i * 4 + 3] = if colorkey && p == 0 { 0 } else { 255 };
        }
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.bg_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(320 * 4),
                rows_per_image: Some(200),
            },
            wgpu::Extent3d { width: 320, height: 200, depth_or_array_layers: 1 },
        );
        // Palette LUT upload (for the hand shader).
        let mut pal = vec![0u8; 256 * 4];
        for (i, c) in palette.iter().enumerate() {
            pal[i * 4..i * 4 + 3].copy_from_slice(c);
            pal[i * 4 + 3] = 255;
        }
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.pal_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pal,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(1024), rows_per_image: Some(1) },
            wgpu::Extent3d { width: 256, height: 1, depth_or_array_layers: 1 },
        );

        // Letterbox geometry (same integer-scale rule as the software path).
        let (ww, wh) = (self.config.width as f32, self.config.height as f32);
        let scale = ((ww / 320.0).min(wh / 200.0)).floor().max(1.0);
        let (dw, dh) = (320.0 * scale, 200.0 * scale);
        let (ox, oy) = (((ww - dw) / 2.0).max(0.0), ((wh - dh) / 2.0).max(0.0));
        let to_ndc = |x: f32, y: f32| -> [f32; 2] {
            [(x / ww) * 2.0 - 1.0, 1.0 - (y / wh) * 2.0]
        };
        let p0 = to_ndc(ox, oy);
        let p1 = to_ndc(ox + dw, oy + dh);
        let quad: [[f32; 4]; 6] = [
            [p0[0], p0[1], 0.0, 0.0],
            [p1[0], p0[1], 1.0, 0.0],
            [p0[0], p1[1], 0.0, 1.0],
            [p1[0], p0[1], 1.0, 0.0],
            [p1[0], p1[1], 1.0, 1.0],
            [p0[0], p1[1], 0.0, 1.0],
        ];
        self.queue
            .write_buffer(&self.quad_vbuf, 0, bytemuck_cast(&quad));

        // Hand triangles: virtual 320x200 space -> letterboxed NDC. Painter order:
        // far-to-near by mean depth (the game's depth-sorted span rule).
        let mut sorted: Vec<&HandTri> = tris.iter().collect();
        sorted.sort_by(|a, b| {
            let da: f32 = a.0.iter().map(|v| v[2]).sum::<f32>();
            let db: f32 = b.0.iter().map(|v| v[2]).sum::<f32>();
            db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut verts: Vec<[f32; 5]> = Vec::with_capacity(tris.len() * 3);
        let max_depth = tris
            .iter()
            .flat_map(|t| t.0.iter().map(|v| v[2]))
            .fold(1.0f32, f32::max);
        for t in sorted {
            for v in t.0 {
                let sx = ox + v[0] * scale;
                let sy = oy + v[1] * scale;
                let ndc = to_ndc(sx, sy);
                verts.push([ndc[0], ndc[1], (v[2] / (max_depth * 2.0)).clamp(0.0, 1.0), v[3], v[4]]);
            }
        }
        let vcount = verts.len().min(self.hand_vcap);
        if vcount > 0 {
            self.queue
                .write_buffer(&self.hand_vbuf, 0, bytemuck_cast(&verts[..vcount]));
        }

        // Star quads: one game pixel each, at the game's projected positions.
        let mut star_verts: Vec<[f32; 3]> = Vec::new();
        for &(sx, sy, shade) in stars {
            let x0 = ox + sx as f32 * scale;
            let y0 = oy + sy as f32 * scale;
            let (x1, y1) = (x0 + scale, y0 + scale);
            let a = to_ndc(x0, y0);
            let b = to_ndc(x1, y0);
            let c = to_ndc(x0, y1);
            let d = to_ndc(x1, y1);
            let sh = f32::from_bits(shade as u32);
            for v in [a, b, c, b, d, c] {
                star_verts.push([v[0], v[1], sh]);
            }
        }
        let star_count = star_verts.len().min(self.star_vcap);
        if star_count > 0 {
            self.queue
                .write_buffer(&self.star_vbuf, 0, bytemuck_cast(&star_verts[..star_count]));
        }

        let frame = self.surface.get_current_texture()?;
        let view = frame.texture.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("present"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if star_count > 0 {
                rp.set_pipeline(&self.star_pipeline);
                rp.set_bind_group(0, &self.star_bind, &[]);
                rp.set_vertex_buffer(0, self.star_vbuf.slice(..));
                rp.draw(0..star_count as u32, 0..1);
            }
            rp.set_pipeline(&self.bg_pipeline);
            rp.set_bind_group(0, &self.bg_bind, &[]);
            rp.set_vertex_buffer(0, self.quad_vbuf.slice(..));
            rp.draw(0..6, 0..1);
            if vcount > 0 {
                rp.set_pipeline(&self.hand_pipeline);
                rp.set_bind_group(0, &self.hand_bind, &[]);
                rp.set_vertex_buffer(0, self.hand_vbuf.slice(..));
                rp.draw(0..vcount as u32, 0..1);
            }
        }
        self.queue.submit([enc.finish()]);
        frame.present();
        Ok(())
    }
}

/// Plain-old-data cast for tightly-packed [f32; N] slices.
fn bytemuck_cast<T: Copy>(data: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data))
    }
}
