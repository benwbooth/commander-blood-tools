//! wgpu presentation of original artwork and recovered native 3D scenes.

use std::borrow::Cow;

use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use commander_blood_formats::alien::AlienAsset;
use commander_blood_formats::manu3::IndexedTexture;
use sdl3::video::Window;

use crate::alien_render::AlienRenderer;
use crate::assets::OriginalFrame;
use crate::bridge_render::BridgeRenderer;
use crate::native::alien::AlienSceneFrame;
use crate::native::bloodprg::{BridgeSceneFrame, IndexedGamePalette};
use crate::native::manu3::model::Manu3Model;
use crate::native::manu3::raster::RenderTriangle;

const MINIMUM_SURFACE_DIMENSION: u32 = 1;
const BASE_MIP_LEVEL: u32 = 0;
const MIP_LEVEL_COUNT: u32 = 1;
const SINGLE_SAMPLE_COUNT: u32 = 1;
const SINGLE_TEXTURE_LAYER: u32 = 1;
const RGBA_BYTES_PER_PIXEL: u32 = 4;
const RGBA_COMPONENT_COUNT: usize = 4;
const TRANSPARENT_ALPHA: u8 = u8::MIN;
const OPAQUE_ALPHA: u8 = u8::MAX;
const VGA_DAC_CHANNEL_MAXIMUM: u16 = 63;
const EIGHT_BIT_CHANNEL_MAXIMUM: u16 = 255;
const IMAGE_TEXTURE_BINDING: u32 = 0;
const IMAGE_SAMPLER_BINDING: u32 = 1;
const DESIRED_FRAME_LATENCY: u32 = 2;
const FULLSCREEN_QUAD_VERTEX_COUNT: u32 = 6;
const MINIMUM_DEPTH: f32 = 0.0;
const MAXIMUM_DEPTH: f32 = 1.0;
const CENTERING_DIVISOR: f32 = 2.0;
const ORIGINAL_DISPLAY_ASPECT_WIDTH: u32 = 4;
const ORIGINAL_DISPLAY_ASPECT_HEIGHT: u32 = 3;
const MANU3_TEXTURE_BINDING: u32 = 0;
const MANU3_VERTEX_COUNT_PER_FACE: usize = 3;
#[cfg(test)]
const PALETTE_ENTRY_COUNT: u32 = 256;
const DEPTH_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const CLEAR_DEPTH: f32 = 1.0;
const EQUAL_DEPTH_VALUE: f32 = 0.5;
const ZERO_DEPTH_RANGE: f64 = 0.0;
const MAXIMUM_BASE_SCENE_RENDERER_COUNT: usize = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BaseSceneArtwork {
    Hide,
    CompositeIndexedOverlay,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArtworkPass {
    OpaqueBase,
    TransparentOverlay,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Manu3GpuVertex {
    screen: [f32; 2],
    texture_coordinates: [f32; 2],
    depth: f32,
}

const MANU3_VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32];

struct Manu3Renderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    maximum_vertex_count: usize,
    depth_view: wgpu::TextureView,
}

/// GPU state for aspect-correct presentation of decoded 2D and 3D content.
pub struct Renderer<'window> {
    surface: wgpu::Surface<'window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    overlay_pipeline: wgpu::RenderPipeline,
    image_bind_group: wgpu::BindGroup,
    image_texture: wgpu::Texture,
    image_width: u32,
    image_height: u32,
    manu3: Option<Manu3Renderer>,
    alien: Option<AlienRenderer>,
    bridge: Option<BridgeRenderer>,
}

impl<'window> Renderer<'window> {
    /// Create a high-performance wgpu device and upload one decoded original frame.
    pub fn new(
        window: &'window Window,
        image: &OriginalFrame,
        manu3_model: Option<&Manu3Model>,
        manu3_palette: Option<&IndexedGamePalette>,
        alien_asset: Option<&AlienAsset>,
        bridge_palette: Option<&IndexedGamePalette>,
    ) -> Result<Self> {
        let base_scene_renderer_count =
            usize::from(alien_asset.is_some()) + usize::from(bridge_palette.is_some());
        if base_scene_renderer_count > MAXIMUM_BASE_SCENE_RENDERER_COUNT {
            anyhow::bail!("only one modern base-scene renderer may be active");
        }
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
        let pipeline = create_artwork_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            format,
            "original artwork pipeline",
            None,
        );
        let overlay_pipeline = create_artwork_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            format,
            "indexed artwork overlay pipeline",
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        let manu3 = manu3_model
            .map(|model| {
                let palette = manu3_palette.context("MANU3 rendering requires authored colors")?;
                Manu3Renderer::new(
                    &device,
                    &queue,
                    format,
                    config.width,
                    config.height,
                    model.texture(),
                    palette,
                    model.faces().len(),
                )
            })
            .transpose()?;
        let alien = alien_asset.map(|asset| {
            AlienRenderer::new(&device, &queue, format, config.width, config.height, asset)
        });
        let bridge = bridge_palette
            .map(|palette| BridgeRenderer::new(&device, format, palette))
            .transpose()?;

        Ok(Self {
            surface,
            device,
            queue,
            config,
            pipeline,
            overlay_pipeline,
            image_bind_group,
            image_texture,
            image_width: image.width,
            image_height: image.height,
            manu3,
            alien,
            bridge,
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
        if let Some(manu3) = &mut self.manu3 {
            manu3.resize(&self.device, width, height);
        }
        if let Some(alien) = &mut self.alien {
            alien.resize(&self.device, width, height);
        }
    }

    /// Install or replace the decoded alien scene used by synchronous overlays.
    ///
    /// Bridge GPU resources remain resident and are selected again as soon as
    /// callers stop supplying alien frames. This mirrors the original overlay
    /// return without rebuilding the bridge panorama or projection state.
    pub fn install_alien_scene(&mut self, asset: &AlienAsset) {
        self.alien = Some(AlienRenderer::new(
            &self.device,
            &self.queue,
            self.config.format,
            self.config.width,
            self.config.height,
            asset,
        ));
    }

    /// Release the temporary alien GPU resources after its XDB loop returns.
    pub fn remove_alien_scene(&mut self) -> bool {
        self.alien.take().is_some()
    }

    /// Upload one complete indexed frame using native six-bit VGA palette values.
    pub fn upload_indexed_frame(
        &self,
        indexed_pixels: &[u8],
        palette: &IndexedGamePalette,
    ) -> Result<()> {
        let expected_pixel_count = usize::try_from(self.image_width)
            .ok()
            .and_then(|width| {
                usize::try_from(self.image_height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .context("artwork texture dimensions exceed the host collection limit")?;
        if indexed_pixels.len() != expected_pixel_count {
            anyhow::bail!(
                "indexed frame has {} pixels; artwork texture requires {expected_pixel_count}",
                indexed_pixels.len()
            );
        }
        let rgba = indexed_frame_rgba(indexed_pixels, palette)?;
        self.upload_rgba_frame(&rgba)
    }

    /// Upload one complete true-color logical frame.
    pub(crate) fn upload_rgba_frame(&self, rgba: &[u8]) -> Result<()> {
        let expected_byte_count = usize::try_from(self.image_width)
            .ok()
            .and_then(|width| {
                usize::try_from(self.image_height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(RGBA_COMPONENT_COUNT))
            .context("artwork texture dimensions exceed the host collection limit")?;
        if rgba.len() != expected_byte_count {
            anyhow::bail!(
                "RGBA frame has {} bytes; artwork texture requires {expected_byte_count}",
                rgba.len()
            );
        }
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.image_texture,
                mip_level: BASE_MIP_LEVEL,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: u64::MIN,
                bytes_per_row: Some(self.image_width * RGBA_BYTES_PER_PIXEL),
                rows_per_image: Some(self.image_height),
            },
            wgpu::Extent3d {
                width: self.image_width,
                height: self.image_height,
                depth_or_array_layers: SINGLE_TEXTURE_LAYER,
            },
        );
        Ok(())
    }

    /// Present artwork or one base scene, followed by the optional MANU3 model.
    ///
    /// An alien or bridge frame replaces the indexed artwork. Use
    /// [`Self::render_with_indexed_overlay`] when the indexed frame contains
    /// game UI that must be composited over the base scene.
    pub fn render(
        &mut self,
        manu3_triangles: &[RenderTriangle],
        alien_frame: Option<&AlienSceneFrame>,
        bridge_frame: Option<&BridgeSceneFrame>,
    ) -> Result<()> {
        self.render_layers(
            manu3_triangles,
            alien_frame,
            bridge_frame,
            BaseSceneArtwork::Hide,
        )
    }

    /// Present one base scene, composite the indexed game UI, then draw MANU3.
    pub fn render_with_indexed_overlay(
        &mut self,
        manu3_triangles: &[RenderTriangle],
        alien_frame: Option<&AlienSceneFrame>,
        bridge_frame: Option<&BridgeSceneFrame>,
    ) -> Result<()> {
        self.render_layers(
            manu3_triangles,
            alien_frame,
            bridge_frame,
            BaseSceneArtwork::CompositeIndexedOverlay,
        )
    }

    /// Refresh bridge foreground colors without recoloring the immutable panorama.
    pub fn update_bridge_actor_palette(&mut self, palette: &IndexedGamePalette) -> Result<()> {
        self.bridge
            .as_mut()
            .context("bridge actor palette supplied without bridge GPU resources")?
            .update_actor_palette(palette)
    }

    fn render_layers(
        &mut self,
        manu3_triangles: &[RenderTriangle],
        alien_frame: Option<&AlienSceneFrame>,
        bridge_frame: Option<&BridgeSceneFrame>,
        base_scene_artwork: BaseSceneArtwork,
    ) -> Result<()> {
        let manu3_vertex_count = match &self.manu3 {
            Some(manu3) => manu3.upload(&self.queue, manu3_triangles)?,
            None if manu3_triangles.is_empty() => u32::MIN,
            None => anyhow::bail!("MANU3 triangles supplied without overlay GPU resources"),
        };
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
            ORIGINAL_DISPLAY_ASPECT_WIDTH,
            ORIGINAL_DISPLAY_ASPECT_HEIGHT,
        );
        if alien_frame.is_some() && bridge_frame.is_some() {
            anyhow::bail!("alien and bridge frames cannot be presented together");
        }
        let base_scene_present = alien_frame.is_some() || bridge_frame.is_some();
        if let Some(alien_frame) = alien_frame {
            let alien = self
                .alien
                .as_mut()
                .context("alien frame supplied without alien GPU resources")?;
            alien.encode(
                &self.queue,
                &mut encoder,
                &view,
                (x, y, width, height),
                alien_frame,
            )?;
        } else if let Some(bridge_frame) = bridge_frame {
            let bridge = self
                .bridge
                .as_mut()
                .context("bridge frame supplied without bridge GPU resources")?;
            bridge.encode(
                &self.queue,
                &mut encoder,
                &view,
                (x, y, width, height),
                bridge_frame,
            )?;
        }
        if let Some(artwork_pass) = artwork_pass(base_scene_present, base_scene_artwork) {
            self.encode_artwork(&mut encoder, &view, (x, y, width, height), artwork_pass);
        }
        if manu3_vertex_count != u32::MIN {
            let manu3 = self
                .manu3
                .as_ref()
                .context("MANU3 vertices were uploaded without GPU resources")?;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Commander Blood MANU3 overlay pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &manu3.depth_view,
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
            pass.set_viewport(x, y, width, height, MINIMUM_DEPTH, MAXIMUM_DEPTH);
            pass.set_pipeline(&manu3.pipeline);
            pass.set_bind_group(MANU3_TEXTURE_BINDING, &manu3.bind_group, &[]);
            pass.set_vertex_buffer(u32::MIN, manu3.vertex_buffer.slice(..));
            pass.draw(u32::MIN..manu3_vertex_count, u32::MIN..SINGLE_TEXTURE_LAYER);
        }
        self.queue.submit([encoder.finish()]);
        frame.present();
        Ok(())
    }

    fn encode_artwork(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        artwork_pass: ArtworkPass,
    ) {
        let overlay = artwork_pass == ArtworkPass::TransparentOverlay;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(if overlay {
                "Commander Blood indexed overlay pass"
            } else {
                "Commander Blood artwork pass"
            }),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: if overlay {
                        wgpu::LoadOp::Load
                    } else {
                        wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                    },
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
        pass.set_pipeline(if overlay {
            &self.overlay_pipeline
        } else {
            &self.pipeline
        });
        pass.set_bind_group(IMAGE_TEXTURE_BINDING, &self.image_bind_group, &[]);
        pass.draw(
            u32::MIN..FULLSCREEN_QUAD_VERTEX_COUNT,
            u32::MIN..SINGLE_TEXTURE_LAYER,
        );
    }
}

const fn artwork_pass(
    base_scene_present: bool,
    base_scene_artwork: BaseSceneArtwork,
) -> Option<ArtworkPass> {
    if !base_scene_present {
        return Some(ArtworkPass::OpaqueBase);
    }
    match base_scene_artwork {
        BaseSceneArtwork::Hide => None,
        BaseSceneArtwork::CompositeIndexedOverlay => Some(ArtworkPass::TransparentOverlay),
    }
}

fn create_artwork_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    label: &'static str,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend,
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
    })
}

/// Expand one complete indexed frame from native six-bit VGA values to RGBA.
pub(crate) fn indexed_frame_rgba(
    indexed_pixels: &[u8],
    palette: &IndexedGamePalette,
) -> Result<Vec<u8>> {
    expand_indexed_rgba(indexed_pixels, palette, true)
}

fn expand_indexed_rgba(
    indexed_pixels: &[u8],
    palette: &IndexedGamePalette,
    transparent_zero: bool,
) -> Result<Vec<u8>> {
    let mut rgba = Vec::with_capacity(indexed_pixels.len() * RGBA_COMPONENT_COUNT);
    for palette_index in indexed_pixels {
        let color = palette[usize::from(*palette_index)];
        for component in color {
            if u16::from(component) > VGA_DAC_CHANNEL_MAXIMUM {
                anyhow::bail!("frame palette component exceeds the six-bit VGA DAC range");
            }
            rgba.push(
                (u16::from(component) * EIGHT_BIT_CHANNEL_MAXIMUM / VGA_DAC_CHANNEL_MAXIMUM) as u8,
            );
        }
        rgba.push(if transparent_zero && *palette_index == u8::MIN {
            TRANSPARENT_ALPHA
        } else {
            OPAQUE_ALPHA
        });
    }
    Ok(rgba)
}

/// Expand the native six-bit VGA palette to packed RGBA entries.
pub(crate) fn indexed_palette_rgba(palette: &IndexedGamePalette) -> Result<Vec<u8>> {
    let indices = (u8::MIN..=u8::MAX).collect::<Vec<_>>();
    expand_indexed_rgba(&indices, palette, false)
}

impl Manu3Renderer {
    #[allow(clippy::too_many_arguments)]
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        surface_width: u32,
        surface_height: u32,
        texture: &IndexedTexture,
        palette: &IndexedGamePalette,
        maximum_triangle_count: usize,
    ) -> Result<Self> {
        let texture_rgba = expand_indexed_rgba(&texture.pixels, palette, false)
            .context("converting the authored MANU3 texture to RGBA")?;
        let texture_size = wgpu::Extent3d {
            width: texture.width as u32,
            height: texture.height as u32,
            depth_or_array_layers: SINGLE_TEXTURE_LAYER,
        };
        let texture_image = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MANU3 RGBA hand texture"),
            size: texture_size,
            mip_level_count: MIP_LEVEL_COUNT,
            sample_count: SINGLE_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
            &texture_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: u64::MIN,
                bytes_per_row: Some(texture.width as u32 * RGBA_BYTES_PER_PIXEL),
                rows_per_image: Some(texture.height as u32),
            },
            texture_size,
        );

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MANU3 RGBA texture layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: MANU3_TEXTURE_BINDING,
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
            label: Some("MANU3 RGBA texture bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: MANU3_TEXTURE_BINDING,
                resource: wgpu::BindingResource::TextureView(
                    &texture_image.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MANU3 RGBA triangle shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("manu3.wgsl"))),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MANU3 pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: u32::MIN,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("MANU3 textured triangle pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Manu3GpuVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &MANU3_VERTEX_ATTRIBUTES,
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
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
        let maximum_vertex_count = maximum_triangle_count * MANU3_VERTEX_COUNT_PER_FACE;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MANU3 dynamic vertices"),
            size: (maximum_vertex_count * size_of::<Manu3GpuVertex>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            pipeline,
            bind_group,
            vertex_buffer,
            maximum_vertex_count,
            depth_view: create_depth_view(device, surface_width, surface_height),
        })
    }

    fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.depth_view = create_depth_view(device, width, height);
    }

    fn upload(&self, queue: &wgpu::Queue, triangles: &[RenderTriangle]) -> Result<u32> {
        let vertex_count = triangles.len() * MANU3_VERTEX_COUNT_PER_FACE;
        if vertex_count > self.maximum_vertex_count {
            anyhow::bail!(
                "MANU3 generated {vertex_count} vertices, exceeding the decoded capacity {}",
                self.maximum_vertex_count
            );
        }
        if triangles.is_empty() {
            return Ok(u32::MIN);
        }

        let mut depths = triangles
            .iter()
            .flat_map(|triangle| triangle.vertices)
            .map(|vertex| vertex.depth);
        let first_depth = depths.next().context("MANU3 triangle has no vertices")?;
        let (minimum_depth, maximum_depth) = depths
            .fold((first_depth, first_depth), |(minimum, maximum), depth| {
                (minimum.min(depth), maximum.max(depth))
            });
        let depth_range = f64::from(maximum_depth) - f64::from(minimum_depth);
        let gpu_vertices = triangles
            .iter()
            .flat_map(|triangle| triangle.vertices)
            .map(|vertex| Manu3GpuVertex {
                screen: vertex.screen.map(f32::from),
                texture_coordinates: vertex.texture.map(f32::from),
                depth: if depth_range == ZERO_DEPTH_RANGE {
                    EQUAL_DEPTH_VALUE
                } else {
                    ((f64::from(vertex.depth) - f64::from(minimum_depth)) / depth_range) as f32
                },
            })
            .collect::<Vec<_>>();
        queue.write_buffer(
            &self.vertex_buffer,
            u64::MIN,
            bytemuck::cast_slice(&gpu_vertices),
        );
        Ok(gpu_vertices.len() as u32)
    }
}

fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("MANU3 depth buffer"),
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

pub(crate) fn aspect_fit_viewport(
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
    use std::path::{Path, PathBuf};

    use commander_blood_formats::manu3::decode_manu3;
    use commander_blood_formats::palette::decode_bloodprg_default_vga_palette;

    use super::*;
    use crate::native::manu3::animation::CursorPosition;
    use crate::native::manu3::model::Manu3FrameRequest;

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
    const OFFSCREEN_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
    const OFFSCREEN_VIEWPORTS: [(u32, u32); 2] = [(640, 360), (256, 384)];
    const MINIMUM_VISIBLE_HAND_PIXELS: usize = 16;
    const CENTERED_CURSOR: CursorPosition = CursorPosition { x: 160, y: 100 };

    fn original_file(candidates: &[&str]) -> Option<PathBuf> {
        candidates
            .iter()
            .map(Path::new)
            .find(|path| path.is_file())
            .map(Path::to_owned)
    }

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

    #[test]
    fn indexed_frame_conversion_expands_native_dac_values_and_rejects_invalid_components() {
        let mut palette = [[u8::MIN; 3]; PALETTE_ENTRY_COUNT as usize];
        palette[1] = [63, 32, 0];
        assert_eq!(
            indexed_frame_rgba(&[1, 0], &palette).unwrap(),
            [255, 129, 0, 255, 0, 0, 0, 0]
        );

        palette[1][0] = 64;
        assert!(indexed_frame_rgba(&[1], &palette).is_err());
    }

    #[test]
    fn indexed_frame_zero_is_transparent_but_palette_entries_remain_opaque() {
        let mut palette = [[u8::MIN; 3]; PALETTE_ENTRY_COUNT as usize];
        palette[usize::from(u8::MIN)] = [63, 32, 0];

        assert_eq!(
            indexed_frame_rgba(&[u8::MIN], &palette).unwrap(),
            [255, 129, 0, TRANSPARENT_ALPHA]
        );
        assert_eq!(
            &indexed_palette_rgba(&palette).unwrap()[..RGBA_COMPONENT_COUNT],
            &[255, 129, 0, OPAQUE_ALPHA]
        );
    }

    #[test]
    fn indexed_artwork_only_overlays_production_base_scenes() {
        assert_eq!(
            artwork_pass(false, BaseSceneArtwork::Hide),
            Some(ArtworkPass::OpaqueBase)
        );
        assert_eq!(artwork_pass(true, BaseSceneArtwork::Hide), None);
        assert_eq!(
            artwork_pass(true, BaseSceneArtwork::CompositeIndexedOverlay),
            Some(ArtworkPass::TransparentOverlay)
        );
    }

    #[test]
    fn original_manu3_renders_nonblank_inside_wide_and_portrait_viewports() {
        let Some(executable_path) = original_file(&[
            "output/_tmp_iso/BLOODPRG.EXE",
            "../../output/_tmp_iso/BLOODPRG.EXE",
        ]) else {
            return;
        };
        let Some(xdb_path) = original_file(&[
            "output/_tmp_dat/manu3.xdb",
            "../../output/_tmp_dat/manu3.xdb",
        ]) else {
            return;
        };
        let palette =
            decode_bloodprg_default_vga_palette(&std::fs::read(executable_path).unwrap()).unwrap();
        let asset = decode_manu3(&std::fs::read(xdb_path).unwrap()).unwrap();
        let mut model = Manu3Model::from_asset(asset).unwrap();
        model
            .render_frame(Manu3FrameRequest {
                cursor: CENTERED_CURSOR,
                animation_selector: u16::MIN,
            })
            .unwrap();
        assert!(!model.render_triangles().is_empty());

        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let Ok(adapter) =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            }))
        else {
            return;
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("MANU3 offscreen test device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            trace: wgpu::Trace::Off,
        }))
        .unwrap();

        for (width, height) in OFFSCREEN_VIEWPORTS {
            assert_offscreen_hand_pixels(&device, &queue, &palette, &model, width, height);
        }
    }

    fn assert_offscreen_hand_pixels(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        palette: &IndexedGamePalette,
        model: &Manu3Model,
        width: u32,
        height: u32,
    ) {
        let renderer = Manu3Renderer::new(
            device,
            queue,
            OFFSCREEN_FORMAT,
            width,
            height,
            model.texture(),
            palette,
            model.faces().len(),
        )
        .unwrap();
        let vertex_count = renderer.upload(queue, model.render_triangles()).unwrap();
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MANU3 offscreen color target"),
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
        let bytes_per_row = width * RGBA_BYTES_PER_PIXEL;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MANU3 offscreen readback"),
            size: u64::from(bytes_per_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MANU3 offscreen encoder"),
        });
        let viewport = aspect_fit_viewport(
            width,
            height,
            ORIGINAL_DISPLAY_ASPECT_WIDTH,
            ORIGINAL_DISPLAY_ASPECT_HEIGHT,
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MANU3 offscreen pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &renderer.depth_view,
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
            pass.set_viewport(
                viewport.0,
                viewport.1,
                viewport.2,
                viewport.3,
                MINIMUM_DEPTH,
                MAXIMUM_DEPTH,
            );
            pass.set_pipeline(&renderer.pipeline);
            pass.set_bind_group(MANU3_TEXTURE_BINDING, &renderer.bind_group, &[]);
            pass.set_vertex_buffer(u32::MIN, renderer.vertex_buffer.slice(..));
            pass.draw(u32::MIN..vertex_count, u32::MIN..SINGLE_TEXTURE_LAYER);
        }
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
        let (viewport_x, viewport_y, viewport_width, viewport_height) = viewport;
        let mut visible_pixel_count = usize::MIN;
        for (index, pixel) in pixels
            .chunks_exact(RGBA_BYTES_PER_PIXEL as usize)
            .enumerate()
        {
            if pixel[..3] == [u8::MIN; 3] {
                continue;
            }
            visible_pixel_count += 1;
            let pixel_x = (index as u32 % width) as f32;
            let pixel_y = (index as u32 / width) as f32;
            assert!(pixel_x >= viewport_x && pixel_x < viewport_x + viewport_width);
            assert!(pixel_y >= viewport_y && pixel_y < viewport_y + viewport_height);
        }
        let screen_bounds = model
            .render_triangles()
            .iter()
            .flat_map(|triangle| triangle.vertices)
            .fold(
                ([i16::MAX; 2], [i16::MIN; 2]),
                |(minimum, maximum), vertex| {
                    (
                        [
                            minimum[0].min(vertex.screen[0]),
                            minimum[1].min(vertex.screen[1]),
                        ],
                        [
                            maximum[0].max(vertex.screen[0]),
                            maximum[1].max(vertex.screen[1]),
                        ],
                    )
                },
            );
        let palette_rgba = indexed_palette_rgba(palette).unwrap();
        let nonblack_palette_entries = palette_rgba
            .chunks_exact(RGBA_BYTES_PER_PIXEL as usize)
            .filter(|entry| entry[..3] != [u8::MIN; 3])
            .count();
        let used_texture_indices = model
            .texture()
            .pixels
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let nonblack_used_indices = used_texture_indices
            .iter()
            .filter(|index| {
                let start = usize::from(**index) * RGBA_BYTES_PER_PIXEL as usize;
                palette_rgba[start..start + 3] != [u8::MIN; 3]
            })
            .count();
        assert!(
            visible_pixel_count >= MINIMUM_VISIBLE_HAND_PIXELS,
            "MANU3 produced {visible_pixel_count} visible pixels from {} triangles, screen bounds {screen_bounds:?}, {nonblack_palette_entries} nonblack palette entries, and {nonblack_used_indices}/{} texture indices with nonblack colors",
            model.render_triangles().len(),
            used_texture_indices.len(),
        );
    }
}
