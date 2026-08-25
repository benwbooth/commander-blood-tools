//! wgpu resources and draw submission for the recovered alien 3D scenes.

use std::borrow::Cow;
use std::mem::size_of;

use anyhow::Result;
use bytemuck::{Pod, Zeroable};
use commander_blood_formats::alien::{AlienAsset, TEXTURE_HEIGHT, TEXTURE_WIDTH};

use crate::native::alien::{AlienRenderTriangle, AlienSceneFrame, AlienStar};

const BASE_MIP_LEVEL: u32 = 0;
const MIP_LEVEL_COUNT: u32 = 1;
const SINGLE_SAMPLE_COUNT: u32 = 1;
const SINGLE_TEXTURE_LAYER: u32 = 1;
const TEXTURE_BINDING: u32 = 0;
const PALETTE_BINDING: u32 = 1;
const PALETTE_ENTRY_COUNT: u32 = 256;
const RGB_COMPONENT_COUNT: usize = 3;
const RGBA_COMPONENT_COUNT: usize = 4;
const ALPHA_COMPONENT: u8 = u8::MAX;
const TRIANGLE_VERTEX_COUNT: usize = 3;
const STAR_VERTEX_COUNT: usize = 6;
const STAR_PIXEL_SIZE: f32 = 1.0;
const MINIMUM_DEPTH: f32 = 0.0;
const MAXIMUM_DEPTH: f32 = 1.0;
const CLEAR_DEPTH: f32 = 1.0;
const EQUAL_DEPTH_VALUE: f32 = 0.5;
const ZERO_DEPTH_RANGE: f64 = 0.0;
const DEPTH_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct AlienGpuVertex {
    screen: [f32; 2],
    texture_coordinates: [f32; 2],
    depth: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct AlienStarGpuVertex {
    screen: [f32; 2],
    palette_index: u32,
}

const ALIEN_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32];
const STAR_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Uint32];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AlienDrawRanges {
    primary_end: u32,
    models_start: u32,
    models_end: u32,
    stars_end: u32,
}

/// GPU resources owned by one decoded AMER, CROOLIS, or SCRUT scene.
pub(crate) struct AlienRenderer {
    textured_pipeline: wgpu::RenderPipeline,
    star_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    indexed_texture: wgpu::Texture,
    indexed_texture_size: wgpu::Extent3d,
    triangle_vertices: wgpu::Buffer,
    star_vertices: wgpu::Buffer,
    maximum_triangle_vertices: usize,
    maximum_star_vertices: usize,
    depth_view: wgpu::TextureView,
}

impl AlienRenderer {
    /// Upload immutable scene textures and create dynamic geometry buffers.
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        surface_width: u32,
        surface_height: u32,
        asset: &AlienAsset,
    ) -> Self {
        let texture_size = wgpu::Extent3d {
            width: asset.texture.width as u32,
            height: asset.texture.height as u32,
            depth_or_array_layers: SINGLE_TEXTURE_LAYER,
        };
        let texture_image = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("alien indexed texture atlas"),
            size: texture_size,
            mip_level_count: MIP_LEVEL_COUNT,
            sample_count: SINGLE_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture_image,
                mip_level: BASE_MIP_LEVEL,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &asset.texture.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: u64::MIN,
                bytes_per_row: Some(asset.texture.width as u32),
                rows_per_image: Some(asset.texture.height as u32),
            },
            texture_size,
        );

        let palette_rgba = palette_rgba(asset);
        let palette_size = wgpu::Extent3d {
            width: PALETTE_ENTRY_COUNT,
            height: SINGLE_TEXTURE_LAYER,
            depth_or_array_layers: SINGLE_TEXTURE_LAYER,
        };
        let palette_image = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("alien display palette"),
            size: palette_size,
            mip_level_count: MIP_LEVEL_COUNT,
            sample_count: SINGLE_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &palette_image,
                mip_level: BASE_MIP_LEVEL,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &palette_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: u64::MIN,
                bytes_per_row: Some(PALETTE_ENTRY_COUNT * RGBA_COMPONENT_COUNT as u32),
                rows_per_image: Some(SINGLE_TEXTURE_LAYER),
            },
            palette_size,
        );

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("alien texture and palette layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: TEXTURE_BINDING,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: PALETTE_BINDING,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("alien texture and palette bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: TEXTURE_BINDING,
                    resource: wgpu::BindingResource::TextureView(
                        &texture_image.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: PALETTE_BINDING,
                    resource: wgpu::BindingResource::TextureView(
                        &palette_image.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("alien indexed geometry shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("alien.wgsl"))),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("alien pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: u32::MIN,
        });
        let textured_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("alien textured triangle pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_textured"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<AlienGpuVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &ALIEN_VERTEX_ATTRIBUTES,
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_textured"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_TEXTURE_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let star_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("alien starfield pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_star"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<AlienStarGpuVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &STAR_VERTEX_ATTRIBUTES,
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_star"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let maximum_triangle_count = asset.primary_model.mesh.faces.len()
            + asset
                .models
                .iter()
                .map(|model| model.mesh.faces.len())
                .sum::<usize>();
        let maximum_triangle_vertices = maximum_triangle_count * TRIANGLE_VERTEX_COUNT;
        let maximum_star_vertices = crate::native::alien::STAR_COUNT * STAR_VERTEX_COUNT;
        let triangle_vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("alien dynamic triangle vertices"),
            size: (maximum_triangle_vertices * size_of::<AlienGpuVertex>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let star_vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("alien dynamic star vertices"),
            size: (maximum_star_vertices * size_of::<AlienStarGpuVertex>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            textured_pipeline,
            star_pipeline,
            bind_group,
            indexed_texture: texture_image,
            indexed_texture_size: texture_size,
            triangle_vertices,
            star_vertices,
            maximum_triangle_vertices,
            maximum_star_vertices,
            depth_view: create_depth_view(device, surface_width, surface_height),
        }
    }

    /// Recreate the depth target after a nonzero surface resize.
    pub(crate) fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.depth_view = create_depth_view(device, width, height);
    }

    /// Upload one recovered scene frame into the dynamic GPU buffers.
    fn upload(&self, queue: &wgpu::Queue, frame: &AlienSceneFrame) -> Result<AlienDrawRanges> {
        if let Some(pixels) = &frame.texture_update {
            let expected = self.indexed_texture_size.width as usize
                * self.indexed_texture_size.height as usize;
            if pixels.len() != expected {
                anyhow::bail!(
                    "alien texture update contains {} bytes, expected {}",
                    pixels.len(),
                    expected
                );
            }
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.indexed_texture,
                    mip_level: BASE_MIP_LEVEL,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: u64::MIN,
                    bytes_per_row: Some(self.indexed_texture_size.width),
                    rows_per_image: Some(self.indexed_texture_size.height),
                },
                self.indexed_texture_size,
            );
        }
        let mut triangle_vertices = Vec::new();
        append_triangle_layer(&mut triangle_vertices, &frame.geometry.primary_triangles);
        let primary_end = triangle_vertices.len();
        append_triangle_layer(&mut triangle_vertices, &frame.geometry.model_triangles);
        if triangle_vertices.len() > self.maximum_triangle_vertices {
            anyhow::bail!(
                "alien scene generated {} triangle vertices, exceeding decoded capacity {}",
                triangle_vertices.len(),
                self.maximum_triangle_vertices
            );
        }
        if !triangle_vertices.is_empty() {
            queue.write_buffer(
                &self.triangle_vertices,
                u64::MIN,
                bytemuck::cast_slice(&triangle_vertices),
            );
        }

        let star_vertices = frame
            .starfield
            .stars
            .iter()
            .flat_map(star_quad)
            .collect::<Vec<_>>();
        if star_vertices.len() > self.maximum_star_vertices {
            anyhow::bail!(
                "alien scene generated {} star vertices, exceeding decoded capacity {}",
                star_vertices.len(),
                self.maximum_star_vertices
            );
        }
        if !star_vertices.is_empty() {
            queue.write_buffer(
                &self.star_vertices,
                u64::MIN,
                bytemuck::cast_slice(&star_vertices),
            );
        }

        Ok(AlienDrawRanges {
            primary_end: primary_end as u32,
            models_start: primary_end as u32,
            models_end: triangle_vertices.len() as u32,
            stars_end: star_vertices.len() as u32,
        })
    }

    /// Encode primary geometry, stars, and model geometry in native frame order.
    pub(crate) fn encode(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        frame: &AlienSceneFrame,
    ) -> Result<()> {
        let ranges = self.upload(queue, frame)?;
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("alien primary geometry pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_DEPTH),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            set_viewport(&mut pass, viewport);
            if ranges.primary_end != u32::MIN {
                pass.set_pipeline(&self.textured_pipeline);
                pass.set_bind_group(TEXTURE_BINDING, &self.bind_group, &[]);
                pass.set_vertex_buffer(u32::MIN, self.triangle_vertices.slice(..));
                pass.draw(u32::MIN..ranges.primary_end, u32::MIN..SINGLE_TEXTURE_LAYER);
            }
        }
        if ranges.stars_end != u32::MIN {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("alien starfield pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
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
            set_viewport(&mut pass, viewport);
            pass.set_pipeline(&self.star_pipeline);
            pass.set_bind_group(TEXTURE_BINDING, &self.bind_group, &[]);
            pass.set_vertex_buffer(u32::MIN, self.star_vertices.slice(..));
            pass.draw(u32::MIN..ranges.stars_end, u32::MIN..SINGLE_TEXTURE_LAYER);
        }
        if ranges.models_end != ranges.models_start {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("alien model geometry pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_DEPTH),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            set_viewport(&mut pass, viewport);
            pass.set_pipeline(&self.textured_pipeline);
            pass.set_bind_group(TEXTURE_BINDING, &self.bind_group, &[]);
            pass.set_vertex_buffer(u32::MIN, self.triangle_vertices.slice(..));
            pass.draw(
                ranges.models_start..ranges.models_end,
                u32::MIN..SINGLE_TEXTURE_LAYER,
            );
        }
        Ok(())
    }
}

fn palette_rgba(asset: &AlienAsset) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(asset.palette.len() * RGBA_COMPONENT_COUNT);
    for color in asset.palette {
        rgba.extend_from_slice(&color[..RGB_COMPONENT_COUNT]);
        rgba.push(ALPHA_COMPONENT);
    }
    rgba
}

fn append_triangle_layer(output: &mut Vec<AlienGpuVertex>, triangles: &[AlienRenderTriangle]) {
    if triangles.is_empty() {
        return;
    }
    let (minimum_depth, maximum_depth) = triangles
        .iter()
        .flat_map(|triangle| triangle.vertices)
        .map(|vertex| vertex.depth)
        .fold((i32::MAX, i32::MIN), |(minimum, maximum), depth| {
            (minimum.min(depth), maximum.max(depth))
        });
    let depth_range = f64::from(maximum_depth) - f64::from(minimum_depth);
    output.extend(
        triangles
            .iter()
            .flat_map(|triangle| triangle.vertices)
            .map(|vertex| AlienGpuVertex {
                screen: vertex.screen.map(f32::from),
                texture_coordinates: vertex.texture.map(f32::from),
                depth: if depth_range == ZERO_DEPTH_RANGE {
                    EQUAL_DEPTH_VALUE
                } else {
                    ((f64::from(vertex.depth) - f64::from(minimum_depth)) / depth_range) as f32
                },
            }),
    );
}

fn star_quad(star: &AlienStar) -> [AlienStarGpuVertex; STAR_VERTEX_COUNT] {
    let left = f32::from(star.screen[0]);
    let top = f32::from(star.screen[1]);
    let right = left + STAR_PIXEL_SIZE;
    let bottom = top + STAR_PIXEL_SIZE;
    let vertex = |screen| AlienStarGpuVertex {
        screen,
        palette_index: u32::from(star.palette_index),
    };
    [
        vertex([left, top]),
        vertex([right, top]),
        vertex([left, bottom]),
        vertex([left, bottom]),
        vertex([right, top]),
        vertex([right, bottom]),
    ]
}

fn set_viewport(pass: &mut wgpu::RenderPass<'_>, viewport: (f32, f32, f32, f32)) {
    pass.set_viewport(
        viewport.0,
        viewport.1,
        viewport.2,
        viewport.3,
        MINIMUM_DEPTH,
        MAXIMUM_DEPTH,
    );
}

fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("alien scene depth buffer"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: SINGLE_TEXTURE_LAYER,
            },
            mip_level_count: MIP_LEVEL_COUNT,
            sample_count: SINGLE_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

const _: () = assert!(TEXTURE_WIDTH == PALETTE_ENTRY_COUNT as usize);
const _: () = assert!(TEXTURE_HEIGHT == PALETTE_ENTRY_COUNT as usize * 2);

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::alien::{AlienXdbKind, decode_alien_xdb};

    use super::*;
    use crate::native::alien::{AlienMouseSample, AlienScene};
    use crate::render::aspect_fit_viewport;

    const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
    const OFFSCREEN_VIEWPORTS: [(u32, u32); 2] = [(640, 360), (256, 384)];
    const ORIGINAL_ASPECT_WIDTH: u32 = 4;
    const ORIGINAL_ASPECT_HEIGHT: u32 = 3;
    const BYTES_PER_PIXEL: u32 = 4;
    const MINIMUM_VISIBLE_PIXELS: usize = 16;
    const CENTERED_MOUSE: AlienMouseSample = AlienMouseSample {
        x: 320,
        y: 512,
        buttons: 0,
    };

    fn original_xdb(name: &str) -> Option<PathBuf> {
        [
            Path::new("output/_tmp_dat").join(name),
            Path::new("../../output/_tmp_dat").join(name),
        ]
        .into_iter()
        .find(|path| path.is_file())
    }

    #[test]
    fn every_original_alien_scene_renders_inside_wide_and_portrait_viewports() {
        let cases = [
            (AlienXdbKind::Amer, "amer.xdb"),
            (AlienXdbKind::Croolis, "croolis.xdb"),
            (AlienXdbKind::Scrut, "scrut.xdb"),
        ];
        let Some((device, queue)) = offscreen_device() else {
            return;
        };

        for (kind, filename) in cases {
            let Some(path) = original_xdb(filename) else {
                continue;
            };
            let asset = decode_alien_xdb(&std::fs::read(path).unwrap(), kind).unwrap();
            let mut scene = AlienScene::from_asset(asset);
            let frame = scene.step(CENTERED_MOUSE).unwrap();
            for (width, height) in OFFSCREEN_VIEWPORTS {
                assert_offscreen_scene(&device, &queue, &scene, &frame, width, height);
            }
        }
    }

    fn offscreen_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .ok()?;
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("alien offscreen test device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            trace: wgpu::Trace::Off,
        }))
        .ok()
    }

    fn assert_offscreen_scene(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &AlienScene,
        frame: &AlienSceneFrame,
        width: u32,
        height: u32,
    ) {
        let renderer = AlienRenderer::new(
            device,
            queue,
            OFFSCREEN_FORMAT,
            width,
            height,
            scene.asset(),
        );
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("alien offscreen color target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: SINGLE_TEXTURE_LAYER,
            },
            mip_level_count: MIP_LEVEL_COUNT,
            sample_count: SINGLE_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: OFFSCREEN_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let output_view = output.create_view(&wgpu::TextureViewDescriptor::default());
        let bytes_per_row = width * BYTES_PER_PIXEL;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("alien offscreen readback"),
            size: u64::from(bytes_per_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("alien offscreen encoder"),
        });
        let viewport =
            aspect_fit_viewport(width, height, ORIGINAL_ASPECT_WIDTH, ORIGINAL_ASPECT_HEIGHT);
        renderer
            .encode(queue, &mut encoder, &output_view, viewport, frame)
            .unwrap();
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &output,
                mip_level: BASE_MIP_LEVEL,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: u64::MIN,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: SINGLE_TEXTURE_LAYER,
            },
        );
        queue.submit([encoder.finish()]);

        let (sender, receiver) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap();
            });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        receiver.recv().unwrap().unwrap();
        let pixels = readback.slice(..).get_mapped_range();
        let mut visible_pixels = usize::MIN;
        for (index, pixel) in pixels.chunks_exact(BYTES_PER_PIXEL as usize).enumerate() {
            if pixel[..RGB_COMPONENT_COUNT] == [u8::MIN; RGB_COMPONENT_COUNT] {
                continue;
            }
            visible_pixels += 1;
            let pixel_x = (index as u32 % width) as f32;
            let pixel_y = (index as u32 / width) as f32;
            assert!(pixel_x >= viewport.0 && pixel_x < viewport.0 + viewport.2);
            assert!(pixel_y >= viewport.1 && pixel_y < viewport.1 + viewport.3);
        }
        assert!(
            visible_pixels >= MINIMUM_VISIBLE_PIXELS,
            "{kind:?} scene produced only {visible_pixels} visible pixels",
            kind = scene.asset().kind,
        );
    }
}
