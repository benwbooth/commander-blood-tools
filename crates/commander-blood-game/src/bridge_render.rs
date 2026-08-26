//! wgpu compositing for the recovered bridge starfield and `TB.BIG` panorama.

use std::borrow::Cow;
use std::mem::size_of;

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};
use commander_blood_formats::panorama::{
    PANORAMA_FRAME_HEIGHT, PANORAMA_FRAME_PIXEL_COUNT, PANORAMA_FRAME_WIDTH,
};

use crate::native::bloodprg::BridgeSceneFrame;
use crate::native::bloodprg::{IndexedGamePalette, SHIP_POINT_CLOUD_COUNT, ShipPlottedPoint};

const BASE_MIP_LEVEL: u32 = 0;
const MIP_LEVEL_COUNT: u32 = 1;
const SINGLE_SAMPLE_COUNT: u32 = 1;
const SINGLE_TEXTURE_LAYER: u32 = 1;
const PANORAMA_TEXTURE_BINDING: u32 = 0;
const PALETTE_TEXTURE_BINDING: u32 = 1;
const RGBA_COMPONENT_COUNT: usize = 4;
const OPAQUE_ALPHA: u8 = u8::MAX;
const VGA_DAC_CHANNEL_MAXIMUM: u16 = 63;
const EIGHT_BIT_CHANNEL_MAXIMUM: u16 = 255;
const STAR_VERTEX_COUNT: usize = 6;
const STAR_PIXEL_SIZE: f32 = 1.0;
const PANORAMA_VERTEX_COUNT: u32 = 3;
const MINIMUM_DEPTH: f32 = 0.0;
const MAXIMUM_DEPTH: f32 = 1.0;
const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BridgeStarGpuVertex {
    screen: [f32; 2],
    palette_index: u32,
}

const STAR_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Uint32];

/// GPU resources for one live bridge panorama and procedural starfield.
pub(crate) struct BridgeRenderer {
    star_pipeline: wgpu::RenderPipeline,
    panorama_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    panorama_texture: wgpu::Texture,
    star_vertices: wgpu::Buffer,
    maximum_star_vertices: usize,
}

impl BridgeRenderer {
    /// Upload the immutable game palette and allocate dynamic bridge resources.
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        palette: &IndexedGamePalette,
    ) -> Result<Self> {
        let panorama_size = wgpu::Extent3d {
            width: PANORAMA_FRAME_WIDTH as u32,
            height: PANORAMA_FRAME_HEIGHT as u32,
            depth_or_array_layers: SINGLE_TEXTURE_LAYER,
        };
        let panorama_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bridge indexed panorama"),
            size: panorama_size,
            mip_level_count: MIP_LEVEL_COUNT,
            sample_count: SINGLE_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let palette_size = wgpu::Extent3d {
            width: PALETTE_ENTRY_COUNT as u32,
            height: SINGLE_TEXTURE_LAYER,
            depth_or_array_layers: SINGLE_TEXTURE_LAYER,
        };
        let palette_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bridge game palette"),
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
                texture: &palette_texture,
                mip_level: BASE_MIP_LEVEL,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &palette_rgba(palette)?,
            wgpu::TexelCopyBufferLayout {
                offset: u64::MIN,
                bytes_per_row: Some(PALETTE_ENTRY_COUNT as u32 * RGBA_COMPONENT_COUNT as u32),
                rows_per_image: Some(SINGLE_TEXTURE_LAYER),
            },
            palette_size,
        );

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bridge indexed layers layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: PANORAMA_TEXTURE_BINDING,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: PALETTE_TEXTURE_BINDING,
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
            label: Some("bridge indexed layers bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: PANORAMA_TEXTURE_BINDING,
                    resource: wgpu::BindingResource::TextureView(
                        &panorama_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: PALETTE_TEXTURE_BINDING,
                    resource: wgpu::BindingResource::TextureView(
                        &palette_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bridge indexed compositor shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("bridge.wgsl"))),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bridge compositor pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: u32::MIN,
        });
        let star_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bridge procedural star pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_star"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<BridgeStarGpuVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &STAR_VERTEX_ATTRIBUTES,
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_indexed"),
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
        let panorama_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bridge transparent panorama pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_panorama"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_panorama"),
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
        let maximum_star_vertices = SHIP_POINT_CLOUD_COUNT * STAR_VERTEX_COUNT;
        let star_vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bridge dynamic star vertices"),
            size: (maximum_star_vertices * size_of::<BridgeStarGpuVertex>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            star_pipeline,
            panorama_pipeline,
            bind_group,
            panorama_texture,
            star_vertices,
            maximum_star_vertices,
        })
    }

    /// Upload and encode one bridge frame in original compositing order.
    pub(crate) fn encode(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        frame: &BridgeSceneFrame,
    ) -> Result<()> {
        let star_vertex_count = self.upload(queue, frame)?;
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bridge procedural starfield pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
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
            set_viewport(&mut pass, viewport);
            if star_vertex_count != u32::MIN {
                pass.set_pipeline(&self.star_pipeline);
                pass.set_bind_group(PANORAMA_TEXTURE_BINDING, &self.bind_group, &[]);
                pass.set_vertex_buffer(u32::MIN, self.star_vertices.slice(..));
                pass.draw(u32::MIN..star_vertex_count, u32::MIN..SINGLE_TEXTURE_LAYER);
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bridge transparent panorama pass"),
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
            pass.set_pipeline(&self.panorama_pipeline);
            pass.set_bind_group(PANORAMA_TEXTURE_BINDING, &self.bind_group, &[]);
            pass.draw(
                u32::MIN..PANORAMA_VERTEX_COUNT,
                u32::MIN..SINGLE_TEXTURE_LAYER,
            );
        }
        Ok(())
    }

    fn upload(&self, queue: &wgpu::Queue, frame: &BridgeSceneFrame) -> Result<u32> {
        if frame.panorama_pixels.len() != PANORAMA_FRAME_PIXEL_COUNT {
            anyhow::bail!(
                "bridge panorama contains {} pixels, expected {}",
                frame.panorama_pixels.len(),
                PANORAMA_FRAME_PIXEL_COUNT
            );
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.panorama_texture,
                mip_level: BASE_MIP_LEVEL,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &frame.panorama_pixels,
            wgpu::TexelCopyBufferLayout {
                offset: u64::MIN,
                bytes_per_row: Some(PANORAMA_FRAME_WIDTH as u32),
                rows_per_image: Some(PANORAMA_FRAME_HEIGHT as u32),
            },
            wgpu::Extent3d {
                width: PANORAMA_FRAME_WIDTH as u32,
                height: PANORAMA_FRAME_HEIGHT as u32,
                depth_or_array_layers: SINGLE_TEXTURE_LAYER,
            },
        );

        let vertices = frame
            .starfield
            .plotted
            .iter()
            .flat_map(star_quad)
            .collect::<Vec<_>>();
        if vertices.len() > self.maximum_star_vertices {
            anyhow::bail!(
                "bridge generated {} star vertices, exceeding point-cloud capacity {}",
                vertices.len(),
                self.maximum_star_vertices
            );
        }
        if !vertices.is_empty() {
            queue.write_buffer(
                &self.star_vertices,
                u64::MIN,
                bytemuck::cast_slice(&vertices),
            );
        }
        u32::try_from(vertices.len()).context("bridge star vertex count exceeds u32")
    }
}

fn palette_rgba(palette: &IndexedGamePalette) -> Result<Vec<u8>> {
    let mut rgba = Vec::with_capacity(PALETTE_ENTRY_COUNT * RGBA_COMPONENT_COUNT);
    for color in palette {
        for component in &color[..RGB_COMPONENT_COUNT] {
            if u16::from(*component) > VGA_DAC_CHANNEL_MAXIMUM {
                anyhow::bail!("bridge palette component exceeds the six-bit VGA DAC range");
            }
            rgba.push(
                (u16::from(*component) * EIGHT_BIT_CHANNEL_MAXIMUM / VGA_DAC_CHANNEL_MAXIMUM) as u8,
            );
        }
        rgba.push(OPAQUE_ALPHA);
    }
    Ok(rgba)
}

fn star_quad(star: &ShipPlottedPoint) -> [BridgeStarGpuVertex; STAR_VERTEX_COUNT] {
    let left = f32::from(star.projection.screen[X_AXIS]);
    let top = f32::from(star.projection.screen[Y_AXIS]);
    let right = left + STAR_PIXEL_SIZE;
    let bottom = top + STAR_PIXEL_SIZE;
    let vertex = |screen| BridgeStarGpuVertex {
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use commander_blood_formats::bloodprg::decode_bloodprg_bridge_resources;
    use commander_blood_formats::palette::decode_bloodprg_default_vga_palette;
    use commander_blood_formats::panorama::BridgePanoramaArchive;

    use super::*;
    use crate::native::bloodprg::{BridgeScene, BridgeSceneInput, ShipProjectionResources};
    use crate::native::random::BloodPrng;
    use crate::render::aspect_fit_viewport;

    const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
    const OFFSCREEN_VIEWPORTS: [(u32, u32); 2] = [(640, 360), (256, 384)];
    const ORIGINAL_ASPECT_WIDTH: u32 = 4;
    const ORIGINAL_ASPECT_HEIGHT: u32 = 3;
    const BYTES_PER_PIXEL: u32 = 4;
    const TEST_CLOCK_BYTE: u8 = 17;
    const MINIMUM_VISIBLE_PIXELS: usize = 1_000;

    #[test]
    fn original_bridge_renders_nonblank_inside_wide_and_portrait_viewports() {
        let Some(executable_path) = original_file("BLOODPRG.EXE") else {
            return;
        };
        let Some(panorama_path) = original_file("TB.BIG") else {
            return;
        };
        let executable = std::fs::read(executable_path).unwrap();
        let resources =
            ShipProjectionResources::from(decode_bloodprg_bridge_resources(&executable).unwrap());
        let palette = decode_bloodprg_default_vga_palette(&executable).unwrap();
        let panorama =
            BridgePanoramaArchive::decode(std::fs::read(panorama_path).unwrap().into_boxed_slice())
                .unwrap();
        let mut random = BloodPrng::default();
        random.seed_from_clock_register(TEST_CLOCK_BYTE);
        let mut scene = BridgeScene::new(panorama, resources, &mut random).unwrap();
        let frame = scene.render_frame(BridgeSceneInput::default()).unwrap();
        let Some((device, queue)) = offscreen_device() else {
            return;
        };

        for (width, height) in OFFSCREEN_VIEWPORTS {
            assert_offscreen_bridge(&device, &queue, &palette, &frame, width, height);
        }
    }

    fn original_file(filename: &str) -> Option<PathBuf> {
        [
            Path::new("output/_tmp_iso").join(filename),
            Path::new("../../output/_tmp_iso").join(filename),
            Path::new("commander-blood-audio/_tmp_iso").join(filename),
            Path::new("../../commander-blood-audio/_tmp_iso").join(filename),
        ]
        .into_iter()
        .find(|path| path.is_file())
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
            label: Some("bridge offscreen test device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            trace: wgpu::Trace::Off,
        }))
        .ok()
    }

    fn assert_offscreen_bridge(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        palette: &IndexedGamePalette,
        frame: &BridgeSceneFrame,
        width: u32,
        height: u32,
    ) {
        let renderer = BridgeRenderer::new(device, queue, OFFSCREEN_FORMAT, palette).unwrap();
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bridge offscreen color target"),
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
            label: Some("bridge offscreen readback"),
            size: u64::from(bytes_per_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("bridge offscreen encoder"),
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
        assert!(visible_pixels >= MINIMUM_VISIBLE_PIXELS);
    }
}
