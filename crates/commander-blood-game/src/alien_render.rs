//! wgpu presentation for the recovered alien software-raster output.
//!
//! Game-visible edge, texture, depth, and layer decisions are resolved by the
//! flat Rust rasterizer. The GPU only scales its 320-by-200 RGBA result into the
//! current aspect-correct viewport.

use std::borrow::Cow;

use anyhow::Result;

use crate::native::alien::AlienSceneFrame;

const BASE_MIP_LEVEL: u32 = 0;
const MIP_LEVEL_COUNT: u32 = 1;
const SINGLE_SAMPLE_COUNT: u32 = 1;
const SINGLE_TEXTURE_LAYER: u32 = 1;
const FRAME_TEXTURE_BINDING: u32 = 0;
const FRAME_WIDTH: u32 = 320;
const FRAME_HEIGHT: u32 = 200;
const RGBA_COMPONENT_COUNT: usize = 4;
const FRAME_VERTEX_COUNT: u32 = 3;
const MINIMUM_DEPTH: f32 = 0.0;
const MAXIMUM_DEPTH: f32 = 1.0;

/// GPU resources for nearest-neighbor presentation of one alien scene frame.
pub(crate) struct AlienRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    frame_texture: wgpu::Texture,
}

impl AlienRenderer {
    /// Create the fixed-size true-color frame texture and presentation pipeline.
    pub(crate) fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let frame_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("alien true-color software frame"),
            size: frame_extent(),
            mip_level_count: MIP_LEVEL_COUNT,
            sample_count: SINGLE_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("alien true-color frame layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: FRAME_TEXTURE_BINDING,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("alien true-color frame bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: FRAME_TEXTURE_BINDING,
                resource: wgpu::BindingResource::TextureView(
                    &frame_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("alien true-color frame shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("alien.wgsl"))),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("alien true-color frame pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: u32::MIN,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("alien true-color frame pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_frame"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_frame"),
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
        Self {
            pipeline,
            bind_group,
            frame_texture,
        }
    }

    /// Upload and present one authoritative software-raster frame.
    pub(crate) fn encode(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        frame: &AlienSceneFrame,
    ) -> Result<()> {
        let expected = FRAME_WIDTH as usize * FRAME_HEIGHT as usize * RGBA_COMPONENT_COUNT;
        if frame.true_color.pixels.len() != expected {
            anyhow::bail!(
                "alien true-color frame contains {} bytes, expected {expected}",
                frame.true_color.pixels.len()
            );
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.frame_texture,
                mip_level: BASE_MIP_LEVEL,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &frame.true_color.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: u64::MIN,
                bytes_per_row: Some(FRAME_WIDTH * RGBA_COMPONENT_COUNT as u32),
                rows_per_image: Some(FRAME_HEIGHT),
            },
            frame_extent(),
        );

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("alien true-color frame pass"),
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
        pass.set_viewport(
            viewport.0,
            viewport.1,
            viewport.2,
            viewport.3,
            MINIMUM_DEPTH,
            MAXIMUM_DEPTH,
        );
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(FRAME_TEXTURE_BINDING, &self.bind_group, &[]);
        pass.draw(u32::MIN..FRAME_VERTEX_COUNT, u32::MIN..SINGLE_TEXTURE_LAYER);
        Ok(())
    }
}

const fn frame_extent() -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: FRAME_WIDTH,
        height: FRAME_HEIGHT,
        depth_or_array_layers: SINGLE_TEXTURE_LAYER,
    }
}

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
    const RGB_COMPONENT_COUNT: usize = 3;
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
                assert_offscreen_scene(&device, &queue, &frame, width, height, kind);
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
            label: Some("alien true-color offscreen device"),
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
        frame: &AlienSceneFrame,
        width: u32,
        height: u32,
        kind: AlienXdbKind,
    ) {
        let mut renderer = AlienRenderer::new(device, OFFSCREEN_FORMAT);
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("alien true-color offscreen target"),
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
            label: Some("alien true-color offscreen readback"),
            size: u64::from(bytes_per_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("alien true-color offscreen encoder"),
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
        );
    }
}
