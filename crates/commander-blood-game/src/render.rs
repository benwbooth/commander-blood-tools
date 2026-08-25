//! wgpu presentation of original two-dimensional artwork.

use std::borrow::Cow;

use anyhow::{Context, Result};
use sdl3::video::Window;

use crate::assets::OriginalFrame;

const MINIMUM_SURFACE_DIMENSION: u32 = 1;
const BASE_MIP_LEVEL: u32 = 0;
const MIP_LEVEL_COUNT: u32 = 1;
const SINGLE_SAMPLE_COUNT: u32 = 1;
const SINGLE_TEXTURE_LAYER: u32 = 1;
const RGBA_BYTES_PER_PIXEL: u32 = 4;
const IMAGE_TEXTURE_BINDING: u32 = 0;
const IMAGE_SAMPLER_BINDING: u32 = 1;
const DESIRED_FRAME_LATENCY: u32 = 2;
const FULLSCREEN_QUAD_VERTEX_COUNT: u32 = 6;
const MINIMUM_DEPTH: f32 = 0.0;
const MAXIMUM_DEPTH: f32 = 1.0;
const CENTERING_DIVISOR: f32 = 2.0;

/// GPU state for aspect-correct presentation of a decoded original frame.
pub struct Renderer<'window> {
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    image_bind_group: wgpu::BindGroup,
    image_size: (u32, u32),
}

impl<'window> Renderer<'window> {
    /// Create a high-performance wgpu device and upload one decoded original frame.
    pub fn new(window: &'window Window, image: &OriginalFrame) -> Result<Self> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = create_surface::create(&instance, window)
            .map_err(anyhow::Error::msg)
            .context("creating wgpu surface for SDL window")?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .context("finding a wgpu graphics adapter")?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Commander Blood device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            trace: wgpu::Trace::Off,
        }))
        .context("creating wgpu device")?;

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(capabilities.formats[BASE_MIP_LEVEL as usize]);
        let (width, height) = window.size_in_pixels();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(MINIMUM_SURFACE_DIMENSION),
            height: height.max(MINIMUM_SURFACE_DIMENSION),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            desired_maximum_frame_latency: DESIRED_FRAME_LATENCY,
            view_formats: Vec::new(),
        };
        surface.configure(&device, &config);

        let image_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("original Commander Blood frame"),
            size: wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: SINGLE_TEXTURE_LAYER,
            },
            mip_level_count: MIP_LEVEL_COUNT,
            sample_count: SINGLE_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &image_texture,
                mip_level: BASE_MIP_LEVEL,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &image.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: u64::MIN,
                bytes_per_row: Some(image.width * RGBA_BYTES_PER_PIXEL),
                rows_per_image: Some(image.height),
            },
            wgpu::Extent3d {
                width: image.width,
                height: image.height,
                depth_or_array_layers: SINGLE_TEXTURE_LAYER,
            },
        );

        let image_view = image_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pixel-accurate nearest-neighbor sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("original frame bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: IMAGE_TEXTURE_BINDING,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: IMAGE_SAMPLER_BINDING,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let image_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("original frame bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: IMAGE_TEXTURE_BINDING,
                    resource: wgpu::BindingResource::TextureView(&image_view),
                },
                wgpu::BindGroupEntry {
                    binding: IMAGE_SAMPLER_BINDING,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("indexed artwork shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("artwork.wgsl"))),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("artwork pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: u32::MIN,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("original artwork pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            image_bind_group,
            image_size: (image.width, image.height),
        })
    }

    /// Reconfigure the presentation surface after a nonzero SDL pixel-size event.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == u32::MIN || height == u32::MIN {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Present the current original frame once.
    pub fn render(&mut self) -> Result<()> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                anyhow::bail!("wgpu rejected the current surface configuration")
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Commander Blood frame encoder"),
            });
        let (x, y, width, height) = aspect_fit_viewport(
            self.config.width,
            self.config.height,
            self.image_size.0,
            self.image_size.1,
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Commander Blood artwork pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            pass.set_viewport(x, y, width, height, MINIMUM_DEPTH, MAXIMUM_DEPTH);
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(IMAGE_TEXTURE_BINDING, &self.image_bind_group, &[]);
            pass.draw(
                u32::MIN..FULLSCREEN_QUAD_VERTEX_COUNT,
                u32::MIN..SINGLE_TEXTURE_LAYER,
            );
        }
        self.queue.submit([encoder.finish()]);
        frame.present();
        Ok(())
    }
}

fn aspect_fit_viewport(
    output_width: u32,
    output_height: u32,
    source_width: u32,
    source_height: u32,
) -> (f32, f32, f32, f32) {
    let scale = (output_width as f32 / source_width as f32)
        .min(output_height as f32 / source_height as f32);
    let width = source_width as f32 * scale;
    let height = source_height as f32 * scale;
    (
        (output_width as f32 - width) / CENTERING_DIVISOR,
        (output_height as f32 - height) / CENTERING_DIVISOR,
        width,
        height,
    )
}

mod create_surface {
    use sdl3::video::Window;
    use wgpu::rwh::{HasDisplayHandle, HasWindowHandle};

    // SDL confines window access to the main thread. This wrapper supplies the
    // Send/Sync contract required while wgpu copies the two raw handles during
    // surface creation; it is never sent to another thread or retained by us.
    struct SyncWindow<'a>(&'a Window);

    unsafe impl Send for SyncWindow<'_> {}
    unsafe impl Sync for SyncWindow<'_> {}

    impl HasWindowHandle for SyncWindow<'_> {
        fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
            self.0.window_handle()
        }
    }

    impl HasDisplayHandle for SyncWindow<'_> {
        fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
            self.0.display_handle()
        }
    }

    pub fn create<'a>(
        instance: &wgpu::Instance,
        window: &'a Window,
    ) -> Result<wgpu::Surface<'a>, String> {
        instance
            .create_surface(SyncWindow(window))
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDESCREEN_WIDTH: u32 = 1920;
    const WIDESCREEN_HEIGHT: u32 = 1080;
    const SQUARE_OUTPUT_SIZE: u32 = 800;
    const SOURCE_WIDTH: u32 = 640;
    const SOURCE_HEIGHT: u32 = 480;
    const WIDESCREEN_EXPECTED_X: f32 = 240.0;
    const WIDESCREEN_EXPECTED_Y: f32 = 0.0;
    const WIDESCREEN_EXPECTED_WIDTH: f32 = 1440.0;
    const WIDESCREEN_EXPECTED_HEIGHT: f32 = 1080.0;
    const SQUARE_EXPECTED_X: f32 = 0.0;
    const SQUARE_EXPECTED_Y: f32 = 100.0;
    const SQUARE_EXPECTED_WIDTH: f32 = 800.0;
    const SQUARE_EXPECTED_HEIGHT: f32 = 600.0;

    #[test]
    fn aspect_fit_preserves_four_by_three() {
        assert_eq!(
            aspect_fit_viewport(
                WIDESCREEN_WIDTH,
                WIDESCREEN_HEIGHT,
                SOURCE_WIDTH,
                SOURCE_HEIGHT
            ),
            (
                WIDESCREEN_EXPECTED_X,
                WIDESCREEN_EXPECTED_Y,
                WIDESCREEN_EXPECTED_WIDTH,
                WIDESCREEN_EXPECTED_HEIGHT
            )
        );
        assert_eq!(
            aspect_fit_viewport(
                SQUARE_OUTPUT_SIZE,
                SQUARE_OUTPUT_SIZE,
                SOURCE_WIDTH,
                SOURCE_HEIGHT
            ),
            (
                SQUARE_EXPECTED_X,
                SQUARE_EXPECTED_Y,
                SQUARE_EXPECTED_WIDTH,
                SQUARE_EXPECTED_HEIGHT
            )
        );
    }
}
