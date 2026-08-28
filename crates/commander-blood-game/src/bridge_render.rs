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
const RGBA_COMPONENT_COUNT: usize = 4;
const OPAQUE_ALPHA: u8 = u8::MAX;
const TRANSPARENT_ALPHA: u8 = u8::MIN;
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
    color: [u8; RGBA_COMPONENT_COUNT],
}

const STAR_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Unorm8x4];

/// GPU resources for one live bridge panorama and procedural starfield.
pub(crate) struct BridgeRenderer {
    star_pipeline: wgpu::RenderPipeline,
    panorama_pipeline: wgpu::RenderPipeline,
    panorama_bind_group: wgpu::BindGroup,
    object_sprite_bind_group: wgpu::BindGroup,
    panorama_texture: wgpu::Texture,
    object_sprite_texture: wgpu::Texture,
    colors: [[u8; RGBA_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
    panorama_rgba: Vec<u8>,
    object_sprite_rgba: Vec<u8>,
    star_vertices: wgpu::Buffer,
    maximum_star_vertices: usize,
}

impl BridgeRenderer {
    /// Resolve authored colors and allocate dynamic true-color bridge resources.
    pub(crate) fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        palette: &IndexedGamePalette,
    ) -> Result<Self> {
        let panorama_size = wgpu::Extent3d {
            width: PANORAMA_FRAME_WIDTH as u32,
            height: PANORAMA_FRAME_HEIGHT as u32,
            depth_or_array_layers: SINGLE_TEXTURE_LAYER,
        };
        let panorama_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bridge RGBA panorama"),
            size: panorama_size,
            mip_level_count: MIP_LEVEL_COUNT,
            sample_count: SINGLE_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let object_sprite_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bridge RGBA object sprites"),
            size: panorama_size,
            mip_level_count: MIP_LEVEL_COUNT,
            sample_count: SINGLE_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let colors = palette_rgba(palette)?;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bridge RGBA layer layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: PANORAMA_TEXTURE_BINDING,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let panorama_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bridge panorama bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: PANORAMA_TEXTURE_BINDING,
                resource: wgpu::BindingResource::TextureView(
                    &panorama_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            }],
        });
        let object_sprite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bridge object sprite bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: PANORAMA_TEXTURE_BINDING,
                resource: wgpu::BindingResource::TextureView(
                    &object_sprite_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bridge true-color compositor shader"),
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
                entry_point: Some("fs_color"),
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
            panorama_bind_group,
            object_sprite_bind_group,
            panorama_texture,
            object_sprite_texture,
            colors,
            panorama_rgba: vec![u8::MIN; PANORAMA_FRAME_PIXEL_COUNT * RGBA_COMPONENT_COUNT],
            object_sprite_rgba: vec![u8::MIN; PANORAMA_FRAME_PIXEL_COUNT * RGBA_COMPONENT_COUNT],
            star_vertices,
            maximum_star_vertices,
        })
    }

    /// Upload and encode one bridge frame in original compositing order.
    pub(crate) fn encode(
        &mut self,
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
                pass.set_bind_group(PANORAMA_TEXTURE_BINDING, &self.panorama_bind_group, &[]);
                pass.set_vertex_buffer(u32::MIN, self.star_vertices.slice(..));
                pass.draw(u32::MIN..star_vertex_count, u32::MIN..SINGLE_TEXTURE_LAYER);
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bridge object sprite and panorama pass"),
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
            pass.set_bind_group(
                PANORAMA_TEXTURE_BINDING,
                &self.object_sprite_bind_group,
                &[],
            );
            pass.draw(
                u32::MIN..PANORAMA_VERTEX_COUNT,
                u32::MIN..SINGLE_TEXTURE_LAYER,
            );
            pass.set_bind_group(PANORAMA_TEXTURE_BINDING, &self.panorama_bind_group, &[]);
            pass.draw(
                u32::MIN..PANORAMA_VERTEX_COUNT,
                u32::MIN..SINGLE_TEXTURE_LAYER,
            );
        }
        Ok(())
    }

    fn upload(&mut self, queue: &wgpu::Queue, frame: &BridgeSceneFrame) -> Result<u32> {
        if frame.panorama_pixels.len() != PANORAMA_FRAME_PIXEL_COUNT {
            anyhow::bail!(
                "bridge panorama contains {} pixels, expected {}",
                frame.panorama_pixels.len(),
                PANORAMA_FRAME_PIXEL_COUNT
            );
        }
        if frame.object_sprite_pixels.len() != PANORAMA_FRAME_PIXEL_COUNT {
            anyhow::bail!(
                "bridge object layer contains {} pixels, expected {}",
                frame.object_sprite_pixels.len(),
                PANORAMA_FRAME_PIXEL_COUNT
            );
        }
        expand_indexed_rgba_into(
            &frame.panorama_pixels,
            &self.colors,
            true,
            &mut self.panorama_rgba,
        );
        expand_indexed_rgba_into(
            &frame.object_sprite_pixels,
            &self.colors,
            true,
            &mut self.object_sprite_rgba,
        );
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.panorama_texture,
                mip_level: BASE_MIP_LEVEL,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.panorama_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: u64::MIN,
                bytes_per_row: Some(PANORAMA_FRAME_WIDTH as u32 * RGBA_COMPONENT_COUNT as u32),
                rows_per_image: Some(PANORAMA_FRAME_HEIGHT as u32),
            },
            wgpu::Extent3d {
                width: PANORAMA_FRAME_WIDTH as u32,
                height: PANORAMA_FRAME_HEIGHT as u32,
                depth_or_array_layers: SINGLE_TEXTURE_LAYER,
            },
        );
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.object_sprite_texture,
                mip_level: BASE_MIP_LEVEL,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.object_sprite_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: u64::MIN,
                bytes_per_row: Some(PANORAMA_FRAME_WIDTH as u32 * RGBA_COMPONENT_COUNT as u32),
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
            .flat_map(|star| star_quad(star, &self.colors))
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

fn palette_rgba(
    palette: &IndexedGamePalette,
) -> Result<[[u8; RGBA_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT]> {
    let mut rgba = [[u8::MIN; RGBA_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
    for (target, color) in rgba.iter_mut().zip(palette) {
        for component in &color[..RGB_COMPONENT_COUNT] {
            if u16::from(*component) > VGA_DAC_CHANNEL_MAXIMUM {
                anyhow::bail!("bridge palette component exceeds the six-bit VGA DAC range");
            }
        }
        for (target_component, component) in target[..RGB_COMPONENT_COUNT]
            .iter_mut()
            .zip(&color[..RGB_COMPONENT_COUNT])
        {
            *target_component =
                (u16::from(*component) * EIGHT_BIT_CHANNEL_MAXIMUM / VGA_DAC_CHANNEL_MAXIMUM) as u8;
        }
        target[RGB_COMPONENT_COUNT] = OPAQUE_ALPHA;
    }
    Ok(rgba)
}

fn expand_indexed_rgba_into(
    pixels: &[u8],
    colors: &[[u8; RGBA_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
    transparent_zero: bool,
    rgba: &mut [u8],
) {
    debug_assert_eq!(rgba.len(), pixels.len() * RGBA_COMPONENT_COUNT);
    for (target, palette_index) in rgba.chunks_exact_mut(RGBA_COMPONENT_COUNT).zip(pixels) {
        let mut color = colors[usize::from(*palette_index)];
        if transparent_zero && *palette_index == u8::MIN {
            color[RGB_COMPONENT_COUNT] = TRANSPARENT_ALPHA;
        }
        target.copy_from_slice(&color);
    }
}

fn star_quad(
    star: &ShipPlottedPoint,
    colors: &[[u8; RGBA_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
) -> [BridgeStarGpuVertex; STAR_VERTEX_COUNT] {
    let left = f32::from(star.projection.screen[X_AXIS]);
    let top = f32::from(star.projection.screen[Y_AXIS]);
    let right = left + STAR_PIXEL_SIZE;
    let bottom = top + STAR_PIXEL_SIZE;
    let vertex = |screen| BridgeStarGpuVertex {
        screen,
        color: colors[usize::from(star.palette_index)],
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
    use crate::native::bloodprg::{
        BRIDGE_SPRITE_ENTITY_COUNT, BridgeScene, BridgeSceneInput, BridgeSpriteEntity,
        ShipProjectionResources,
    };
    use crate::native::random::BloodPrng;
    use crate::render::aspect_fit_viewport;

    const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
    const OFFSCREEN_VIEWPORTS: [(u32, u32); 2] = [(640, 360), (256, 384)];
    const ORIGINAL_ASPECT_WIDTH: u32 = 4;
    const ORIGINAL_ASPECT_HEIGHT: u32 = 3;
    const BYTES_PER_PIXEL: u32 = 4;
    const TEST_CLOCK_BYTE: u8 = 17;
    const MINIMUM_VISIBLE_PIXELS: usize = 1_000;
    const LOGICAL_OUTPUT_WIDTH: u32 = PANORAMA_FRAME_WIDTH as u32;
    const LOGICAL_OUTPUT_HEIGHT: u32 = PANORAMA_FRAME_HEIGHT as u32;
    const OBJECT_SAMPLE_X: usize = PANORAMA_FRAME_WIDTH / 2;
    const OBJECT_SAMPLE_Y: usize = PANORAMA_FRAME_HEIGHT / 2;
    const OBJECT_SAMPLE_PALETTE_INDEX: u8 = 17;
    const PANORAMA_SAMPLE_PALETTE_INDEX: u8 = 23;
    const RED_DAC_COLOR: [u8; RGB_COMPONENT_COUNT] = [63, u8::MIN, u8::MIN];
    const GREEN_DAC_COLOR: [u8; RGB_COMPONENT_COUNT] = [u8::MIN, 63, u8::MIN];
    const RED_RGBA_COLOR: [u8; RGBA_COMPONENT_COUNT] = [u8::MAX, u8::MIN, u8::MIN, u8::MAX];
    const GREEN_RGBA_COLOR: [u8; RGBA_COMPONENT_COUNT] = [u8::MIN, u8::MAX, u8::MIN, u8::MAX];

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
        let mut sprite_entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
        let frame = scene
            .render_frame(BridgeSceneInput::default(), &mut sprite_entities)
            .unwrap();
        let Some((device, queue)) = offscreen_device() else {
            return;
        };

        for (width, height) in OFFSCREEN_VIEWPORTS {
            assert_offscreen_bridge(&device, &queue, &palette, &frame, width, height);
        }
    }

    #[test]
    fn object_sprite_layer_is_visible_through_transparent_panorama_and_below_opaque_panorama() {
        let Some(executable_path) = original_file("BLOODPRG.EXE") else {
            return;
        };
        let Some(panorama_path) = original_file("TB.BIG") else {
            return;
        };
        let executable = std::fs::read(executable_path).unwrap();
        let resources =
            ShipProjectionResources::from(decode_bloodprg_bridge_resources(&executable).unwrap());
        let panorama =
            BridgePanoramaArchive::decode(std::fs::read(panorama_path).unwrap().into_boxed_slice())
                .unwrap();
        let mut random = BloodPrng::default();
        random.seed_from_clock_register(TEST_CLOCK_BYTE);
        let mut scene = BridgeScene::new(panorama, resources, &mut random).unwrap();
        let mut sprite_entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
        let mut frame = scene
            .render_frame(BridgeSceneInput::default(), &mut sprite_entities)
            .unwrap();
        frame.starfield.plotted = Box::default();
        frame.panorama_pixels.fill(u8::MIN);
        frame.object_sprite_pixels.fill(u8::MIN);
        let sample_index = OBJECT_SAMPLE_Y * PANORAMA_FRAME_WIDTH + OBJECT_SAMPLE_X;
        frame.object_sprite_pixels[sample_index] = OBJECT_SAMPLE_PALETTE_INDEX;

        let mut palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        palette[usize::from(OBJECT_SAMPLE_PALETTE_INDEX)] = RED_DAC_COLOR;
        palette[usize::from(PANORAMA_SAMPLE_PALETTE_INDEX)] = GREEN_DAC_COLOR;
        let Some((device, queue)) = offscreen_device() else {
            return;
        };

        let object_only = render_offscreen_bridge(
            &device,
            &queue,
            &palette,
            &frame,
            LOGICAL_OUTPUT_WIDTH,
            LOGICAL_OUTPUT_HEIGHT,
        );
        assert_eq!(rgba_at(&object_only, sample_index), RED_RGBA_COLOR);

        frame.panorama_pixels[sample_index] = PANORAMA_SAMPLE_PALETTE_INDEX;
        let panorama_over_object = render_offscreen_bridge(
            &device,
            &queue,
            &palette,
            &frame,
            LOGICAL_OUTPUT_WIDTH,
            LOGICAL_OUTPUT_HEIGHT,
        );
        assert_eq!(
            rgba_at(&panorama_over_object, sample_index),
            GREEN_RGBA_COLOR
        );
    }

    fn original_file(filename: &str) -> Option<PathBuf> {
        [
            Path::new("output/_tmp_iso").join(filename),
            Path::new("../../output/_tmp_iso").join(filename),
            Path::new("commander-blood-audio/_tmp_iso").join(filename),
            Path::new("../../commander-blood-audio/_tmp_iso").join(filename),
            Path::new("re/bin").join(filename),
            Path::new("../../re/bin").join(filename),
            Path::new("accuracy/cblood_install/cblood").join(filename),
            Path::new("../../accuracy/cblood_install/cblood").join(filename),
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
        let pixels = render_offscreen_bridge(device, queue, palette, frame, width, height);
        let viewport =
            aspect_fit_viewport(width, height, ORIGINAL_ASPECT_WIDTH, ORIGINAL_ASPECT_HEIGHT);
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

    fn render_offscreen_bridge(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        palette: &IndexedGamePalette,
        frame: &BridgeSceneFrame,
        width: u32,
        height: u32,
    ) -> Vec<u8> {
        let mut renderer = BridgeRenderer::new(device, OFFSCREEN_FORMAT, palette).unwrap();
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
        let owned_pixels = pixels.to_vec();
        drop(pixels);
        readback.unmap();
        owned_pixels
    }

    fn rgba_at(pixels: &[u8], pixel_index: usize) -> [u8; RGBA_COMPONENT_COUNT] {
        let byte_index = pixel_index * RGBA_COMPONENT_COUNT;
        pixels[byte_index..byte_index + RGBA_COMPONENT_COUNT]
            .try_into()
            .unwrap()
    }
}
