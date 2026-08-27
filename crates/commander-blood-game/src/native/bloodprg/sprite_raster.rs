//! Concrete bridge sprite dispatch over resolved resources and flat surfaces.

use std::error::Error;
use std::fmt;
use std::ops::RangeInclusive;

use super::{
    BridgeSpriteBlitError, BridgeSpriteBlitterMode, BridgeSpriteEntity, BridgeSpriteFrameSource,
    BridgeSpriteRangeError, BridgeSpriteRect, BridgeSpriteRemapTables, BridgeSpriteRenderOutcome,
    ResourceId, blit_raw_opaque_sprite, blit_raw_transparent_sprite,
    blit_retained_framebuffer_sprite, blit_rle_opaque_sprite, blit_rle_transparent_sprite,
    blit_scaled_transparent_sprite, render_bridge_sprite_dirty_range,
};

/// Failure while resolving or rasterizing one concrete sprite dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeSpriteRasterError {
    /// The requested entity range is outside the shared table.
    Range(BridgeSpriteRangeError),
    /// An active entity references a resource that is not cache-owned.
    MissingCachedResource {
        /// Entity that selected the missing resource.
        entity_index: usize,
        /// Stable authored resource identity.
        resource: ResourceId,
    },
    /// A draw request selected an authored no-operation dispatch slot.
    AuthoredNoOperationDispatched {
        /// Entity that produced the impossible request.
        entity_index: usize,
        /// Reserved original dispatch slot.
        mode: BridgeSpriteBlitterMode,
    },
    /// One checked sprite blitter rejected its resource or geometry.
    Blit {
        /// Entity being rasterized when validation failed.
        entity_index: usize,
        /// Concrete raster validation failure.
        source: BridgeSpriteBlitError,
    },
}

impl fmt::Display for BridgeSpriteRasterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "bridge sprite rasterization failed: {self:?}")
    }
}

impl Error for BridgeSpriteRasterError {}

impl From<BridgeSpriteRangeError> for BridgeSpriteRasterError {
    fn from(error: BridgeSpriteRangeError) -> Self {
        Self::Range(error)
    }
}

/// Concrete result of one transactional dirty-range raster pass.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BridgeSpriteRasterOutcome {
    /// Recovered reverse-order entity and dirty-region dispatch.
    pub dispatch: BridgeSpriteRenderOutcome,
    /// Draw requests successfully applied to the flat destination.
    pub rasterized_request_count: usize,
}

/// Flat surfaces, clipping, and color tables used by one raster pass.
pub struct BridgeSpriteRasterTarget<'a> {
    /// Ordered dirty rectangles tested against each active entity.
    pub dirty_regions: &'a [BridgeSpriteRect],
    /// Retained secondary page bound by the dynamic background entity.
    pub retained_framebuffer: &'a [u8],
    /// Indexed destination receiving the completed sprite layer.
    pub framebuffer: &'a mut [u8],
    /// Both destination-color tables built during bridge screen setup.
    pub remap_tables: BridgeSpriteRemapTables<'a>,
}

/// Resolve and rasterize every draw request produced by one sprite range.
///
/// The entity table and destination are staged together, so a missing resource
/// or malformed frame cannot leave dirty flags cleared after a partial frame.
/// Resource IDs resolve to immutable owned bytes, and the retained bridge page
/// is supplied as an ordinary independent surface.
pub fn rasterize_bridge_sprite_range<'resource>(
    entities: &mut [BridgeSpriteEntity],
    entity_range: RangeInclusive<usize>,
    resolve_resource: impl Fn(ResourceId) -> Option<&'resource [u8]>,
    target: BridgeSpriteRasterTarget<'_>,
) -> Result<BridgeSpriteRasterOutcome, BridgeSpriteRasterError> {
    let first_entity_index = *entity_range.start();
    let last_entity_index = *entity_range.end();
    let mut staged_entities = entities.to_vec();
    let mut staged_framebuffer = target.framebuffer.to_vec();
    let dispatch = render_bridge_sprite_dirty_range(
        &mut staged_entities,
        first_entity_index,
        last_entity_index,
        target.dirty_regions,
    )?;

    for request in dispatch.draw_requests.iter().copied() {
        let mut entity = staged_entities[request.entity_index];
        entity.dirty_region = Some(request.dirty_region);
        let frame = entity.frame.ok_or(BridgeSpriteRasterError::Blit {
            entity_index: request.entity_index,
            source: BridgeSpriteBlitError::MissingFrame,
        })?;
        match frame.source {
            BridgeSpriteFrameSource::CachedResource { resource, .. } => {
                let bytes = resolve_resource(resource).ok_or(
                    BridgeSpriteRasterError::MissingCachedResource {
                        entity_index: request.entity_index,
                        resource,
                    },
                )?;
                rasterize_cached_request(
                    request.entity_index,
                    &entity,
                    request.selection,
                    bytes,
                    &mut staged_framebuffer,
                    target.remap_tables,
                )?;
            }
            BridgeSpriteFrameSource::RetainedFramebuffer => {
                blit_retained_framebuffer_sprite(
                    &entity,
                    request.selection,
                    target.retained_framebuffer,
                    &mut staged_framebuffer,
                    target.remap_tables,
                )
                .map_err(|source| BridgeSpriteRasterError::Blit {
                    entity_index: request.entity_index,
                    source,
                })?;
            }
        }
    }

    entities.copy_from_slice(&staged_entities);
    target.framebuffer.copy_from_slice(&staged_framebuffer);
    let rasterized_request_count = dispatch.draw_requests.len();
    Ok(BridgeSpriteRasterOutcome {
        dispatch,
        rasterized_request_count,
    })
}

fn rasterize_cached_request(
    entity_index: usize,
    entity: &BridgeSpriteEntity,
    selection: super::BridgeSpriteBlitterSelection,
    resource_bytes: &[u8],
    framebuffer: &mut [u8],
    remap_tables: BridgeSpriteRemapTables<'_>,
) -> Result<(), BridgeSpriteRasterError> {
    let result = match selection.mode {
        BridgeSpriteBlitterMode::RawTransparent => blit_raw_transparent_sprite(
            entity,
            selection,
            resource_bytes,
            framebuffer,
            remap_tables,
        )
        .map(|_| ()),
        BridgeSpriteBlitterMode::RleTransparent => blit_rle_transparent_sprite(
            entity,
            selection,
            resource_bytes,
            framebuffer,
            remap_tables,
        )
        .map(|_| ()),
        BridgeSpriteBlitterMode::RawOpaque => {
            blit_raw_opaque_sprite(entity, selection, resource_bytes, framebuffer).map(|_| ())
        }
        BridgeSpriteBlitterMode::RleOpaque => {
            blit_rle_opaque_sprite(entity, selection, resource_bytes, framebuffer).map(|_| ())
        }
        BridgeSpriteBlitterMode::ScaledTransparent => {
            blit_scaled_transparent_sprite(entity, resource_bytes, framebuffer).map(|_| ())
        }
        mode => {
            return Err(BridgeSpriteRasterError::AuthoredNoOperationDispatched {
                entity_index,
                mode,
            });
        }
    };
    result.map_err(|source| BridgeSpriteRasterError::Blit {
        entity_index,
        source,
    })
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::lbm::PALETTE_ENTRY_COUNT;

    use super::*;
    use crate::native::bloodprg::{
        BRIDGE_SPRITE_ENTITY_COUNT, BridgeSpriteExtent, BridgeSpriteFlags,
        BridgeSpriteFrameReference, BridgeSpritePosition,
        activate_bridge_sprite_from_retained_framebuffer,
    };

    const ACTIVE_VISIBLE_FLAGS: u16 = 129;
    const RAW_SOURCE_WIDTH: u16 = 2;
    const RAW_SOURCE_HEIGHT: u16 = 2;
    const RAW_DRAW_X: u16 = 10;
    const RAW_DRAW_Y: u16 = 20;
    const RAW_PIXELS: [u8; 4] = [7, 0, 11, 13];
    const TEST_RESOURCE: ResourceId = ResourceId::new(3);
    const FRAME_HEADER_BYTE_COUNT: usize = 8;
    const LOGICAL_WIDTH: usize = 320;
    const LOGICAL_HEIGHT: usize = 200;
    const FRAME_ORIGIN: i16 = 0;
    const LOGICAL_ORIGIN: i32 = 0;
    const RETAINED_SAMPLE_X: usize = 5;
    const RETAINED_SAMPLE_Y: usize = 6;
    const RETAINED_SAMPLE_COLOR: u8 = 47;
    const SINGLE_PIXEL_EXTENT: i32 = 1;
    const TEST_ENTITY_INDEX: usize = usize::MIN;
    const EXPECTED_DRAW_REQUEST_COUNT: usize = 1;

    #[test]
    fn cached_resource_dispatch_draws_the_selected_raw_frame() {
        let mut resource = Vec::from(RAW_SOURCE_WIDTH.to_le_bytes());
        resource.extend_from_slice(&RAW_SOURCE_HEIGHT.to_le_bytes());
        resource.extend_from_slice(&FRAME_ORIGIN.to_le_bytes());
        resource.extend_from_slice(&FRAME_ORIGIN.to_le_bytes());
        resource.extend_from_slice(&RAW_PIXELS);
        assert_eq!(resource.len(), FRAME_HEADER_BYTE_COUNT + RAW_PIXELS.len());

        let mut entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
        entities[TEST_ENTITY_INDEX] = BridgeSpriteEntity {
            flags: BridgeSpriteFlags::from_bits(ACTIVE_VISIBLE_FLAGS),
            frame: Some(BridgeSpriteFrameReference {
                source: BridgeSpriteFrameSource::CachedResource {
                    resource: TEST_RESOURCE,
                    byte_offset: usize::MIN,
                },
                frame_index: usize::MIN,
            }),
            source_extent: BridgeSpriteExtent {
                width: RAW_SOURCE_WIDTH,
                height: RAW_SOURCE_HEIGHT,
            },
            draw_position: BridgeSpritePosition {
                x: RAW_DRAW_X,
                y: RAW_DRAW_Y,
            },
            extent: BridgeSpriteExtent {
                width: RAW_SOURCE_WIDTH,
                height: RAW_SOURCE_HEIGHT,
            },
            ..BridgeSpriteEntity::default()
        };
        let dirty = [full_display_clip()];
        let retained = vec![u8::MIN; LOGICAL_WIDTH * LOGICAL_HEIGHT];
        let mut destination = retained.clone();
        let identity = identity_remap();

        let outcome = rasterize_bridge_sprite_range(
            &mut entities,
            TEST_ENTITY_INDEX..=TEST_ENTITY_INDEX,
            |resource_id| (resource_id == TEST_RESOURCE).then_some(resource.as_slice()),
            BridgeSpriteRasterTarget {
                dirty_regions: &dirty,
                retained_framebuffer: &retained,
                framebuffer: &mut destination,
                remap_tables: BridgeSpriteRemapTables {
                    first: &identity,
                    second: &identity,
                },
            },
        )
        .unwrap();

        assert_eq!(
            outcome.rasterized_request_count,
            EXPECTED_DRAW_REQUEST_COUNT
        );
        let first = usize::from(RAW_DRAW_Y) * LOGICAL_WIDTH + usize::from(RAW_DRAW_X);
        assert_eq!(destination[first], RAW_PIXELS[0]);
        assert_eq!(destination[first + 1], u8::MIN);
        assert_eq!(destination[first + LOGICAL_WIDTH], RAW_PIXELS[2]);
        assert_eq!(destination[first + LOGICAL_WIDTH + 1], RAW_PIXELS[3]);
    }

    #[test]
    fn missing_cache_entry_leaves_entities_and_destination_unchanged() {
        let mut entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
        entities[TEST_ENTITY_INDEX] = BridgeSpriteEntity {
            flags: BridgeSpriteFlags::from_bits(ACTIVE_VISIBLE_FLAGS),
            frame: Some(BridgeSpriteFrameReference {
                source: BridgeSpriteFrameSource::CachedResource {
                    resource: TEST_RESOURCE,
                    byte_offset: usize::MIN,
                },
                frame_index: usize::MIN,
            }),
            source_extent: BridgeSpriteExtent {
                width: RAW_SOURCE_WIDTH,
                height: RAW_SOURCE_HEIGHT,
            },
            draw_position: BridgeSpritePosition {
                x: RAW_DRAW_X,
                y: RAW_DRAW_Y,
            },
            extent: BridgeSpriteExtent {
                width: RAW_SOURCE_WIDTH,
                height: RAW_SOURCE_HEIGHT,
            },
            ..BridgeSpriteEntity::default()
        };
        let entities_before = entities;
        let retained = vec![u8::MIN; LOGICAL_WIDTH * LOGICAL_HEIGHT];
        let mut destination = retained.clone();
        let destination_before = destination.clone();
        let identity = identity_remap();

        let error = rasterize_bridge_sprite_range(
            &mut entities,
            TEST_ENTITY_INDEX..=TEST_ENTITY_INDEX,
            |_| None::<&[u8]>,
            BridgeSpriteRasterTarget {
                dirty_regions: &[full_display_clip()],
                retained_framebuffer: &retained,
                framebuffer: &mut destination,
                remap_tables: BridgeSpriteRemapTables {
                    first: &identity,
                    second: &identity,
                },
            },
        )
        .unwrap_err();

        assert_eq!(
            error,
            BridgeSpriteRasterError::MissingCachedResource {
                entity_index: TEST_ENTITY_INDEX,
                resource: TEST_RESOURCE,
            }
        );
        assert_eq!(entities, entities_before);
        assert_eq!(destination, destination_before);
    }

    #[test]
    fn retained_framebuffer_dispatch_reads_the_flat_secondary_surface() {
        let mut entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
        activate_bridge_sprite_from_retained_framebuffer(
            &mut entities,
            TEST_ENTITY_INDEX,
            BridgeSpriteExtent {
                width: LOGICAL_WIDTH as u16,
                height: LOGICAL_HEIGHT as u16,
            },
            BridgeSpritePosition::default(),
        )
        .unwrap();
        let mut retained = vec![u8::MIN; LOGICAL_WIDTH * LOGICAL_HEIGHT];
        let sample_index = RETAINED_SAMPLE_Y * LOGICAL_WIDTH + RETAINED_SAMPLE_X;
        retained[sample_index] = RETAINED_SAMPLE_COLOR;
        let mut destination = vec![u8::MIN; LOGICAL_WIDTH * LOGICAL_HEIGHT];
        let identity = identity_remap();
        let dirty = [BridgeSpriteRect {
            left: RETAINED_SAMPLE_X as i32,
            right: RETAINED_SAMPLE_X as i32 + SINGLE_PIXEL_EXTENT,
            top: RETAINED_SAMPLE_Y as i32,
            bottom: RETAINED_SAMPLE_Y as i32 + SINGLE_PIXEL_EXTENT,
        }];

        let outcome = rasterize_bridge_sprite_range(
            &mut entities,
            TEST_ENTITY_INDEX..=TEST_ENTITY_INDEX,
            |_| None::<&[u8]>,
            BridgeSpriteRasterTarget {
                dirty_regions: &dirty,
                retained_framebuffer: &retained,
                framebuffer: &mut destination,
                remap_tables: BridgeSpriteRemapTables {
                    first: &identity,
                    second: &identity,
                },
            },
        )
        .unwrap();

        assert_eq!(
            outcome.rasterized_request_count,
            EXPECTED_DRAW_REQUEST_COUNT
        );
        assert_eq!(destination[sample_index], RETAINED_SAMPLE_COLOR);
    }

    fn full_display_clip() -> BridgeSpriteRect {
        BridgeSpriteRect {
            left: LOGICAL_ORIGIN,
            right: LOGICAL_WIDTH as i32,
            top: LOGICAL_ORIGIN,
            bottom: LOGICAL_HEIGHT as i32,
        }
    }

    fn identity_remap() -> [u8; PALETTE_ENTRY_COUNT] {
        std::array::from_fn(|index| index as u8)
    }
}
