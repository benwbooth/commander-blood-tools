//! Shared bridge sprite geometry updates over a typed entity table.

use std::error::Error;
use std::fmt;

use super::resource_cache::{OriginalResourceCache, ResourceId};

/// Number of records in the recovered bridge sprite entity table.
pub const BRIDGE_SPRITE_ENTITY_COUNT: usize = 32;

const STATE_ZERO_FLAG: u16 = 1;
const DIRTY_FLAG: u16 = 2;
const EXTENT_CHANGED_FLAG: u16 = 16;
const RESOURCE_FRAME_ENCODING_FLAG: u16 = 4;
const VISIBLE_FLAG: u16 = 128;
const GEOMETRY_UPDATE_MASK: u16 = STATE_ZERO_FLAG | VISIBLE_FLAG;
const ACTIVATED_FLAGS: u16 = STATE_ZERO_FLAG | DIRTY_FLAG | VISIBLE_FLAG;
const RESOURCE_HEADER_BYTE_COUNT: usize = 4;
const RESOURCE_FRAME_TABLE_ENTRY_BYTE_COUNT: usize = size_of::<u32>();
const SPRITE_FRAME_HEADER_BYTE_COUNT: usize = 8;

/// Recovered bridge sprite flags with named geometry behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BridgeSpriteFlags(u16);

impl BridgeSpriteFlags {
    /// Retain every recovered flag bit, including currently unnamed high bits.
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    /// Return the complete recovered flag word.
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Whether the sprite participates in bridge object projection.
    pub const fn is_visible(self) -> bool {
        self.0 & VISIBLE_FLAG != u16::MIN
    }

    /// Whether position and extent setters accept changes for this entity.
    pub const fn accepts_geometry_updates(self) -> bool {
        self.0 & GEOMETRY_UPDATE_MASK != u16::MIN
    }

    /// Whether current geometry must be committed or redrawn.
    pub const fn is_dirty(self) -> bool {
        self.0 & DIRTY_FLAG != u16::MIN
    }

    /// Whether the current extent differs from the comparison source.
    pub const fn has_scaled_extent(self) -> bool {
        self.0 & EXTENT_CHANGED_FLAG != u16::MIN
    }

    fn mark_dirty(&mut self) {
        self.0 |= DIRTY_FLAG;
    }

    fn transition_active_slot_to_dirty(&mut self) -> bool {
        if !self.is_visible() {
            return false;
        }
        self.0 = self.0 & !(STATE_ZERO_FLAG | VISIBLE_FLAG) | DIRTY_FLAG;
        true
    }

    fn advance_state_zero(&mut self) -> bool {
        if !self.is_visible() || self.0 & STATE_ZERO_FLAG == u16::MIN {
            return false;
        }
        self.0 = self.0 & !STATE_ZERO_FLAG | DIRTY_FLAG;
        true
    }

    fn mark_scaled_extent(&mut self) {
        self.0 |= EXTENT_CHANGED_FLAG | DIRTY_FLAG;
    }

    fn clear_scaled_extent(&mut self) -> bool {
        let changed = self.has_scaled_extent();
        self.0 &= !EXTENT_CHANGED_FLAG;
        if changed {
            self.mark_dirty();
        }
        changed
    }
}

/// Width and height of a decoded bridge sprite frame or destination.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BridgeSpriteExtent {
    /// Horizontal pixel count.
    pub width: u16,
    /// Vertical pixel count.
    pub height: u16,
}

/// Logical draw position of one bridge sprite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BridgeSpritePosition {
    /// Left edge in logical screen coordinates.
    pub x: u16,
    /// Top edge in logical screen coordinates.
    pub y: u16,
}

/// Stable location of a decoded sprite frame in the flat resource cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeSpriteFrameReference {
    /// Original resource catalog identifier owning the frame bytes.
    pub resource: ResourceId,
    /// Zero-based frame selected from the resource directory.
    pub frame_index: usize,
    /// Byte offset of the frame header from the start of the resource.
    pub byte_offset: usize,
}

/// Render-facing geometry owned by one bridge sprite entity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BridgeSpriteEntity {
    /// Recovered state, visibility, dirty, and extent flags.
    pub flags: BridgeSpriteFlags,
    /// Decoded source frame selected for this entity.
    pub frame: Option<BridgeSpriteFrameReference>,
    /// Dimensions of the decoded source frame used for perspective scaling.
    pub source_extent: BridgeSpriteExtent,
    /// Current logical draw position.
    pub draw_position: BridgeSpritePosition,
    /// Current logical destination extent.
    pub extent: BridgeSpriteExtent,
    /// Last rendered extent, initialized independently for each dimension.
    pub committed_extent: BridgeSpriteExtent,
}

/// Invalid entity activation request or malformed sprite resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeSpriteActivationError {
    /// The selected entity does not exist in the typed table.
    Entity(BridgeSpriteEntityError),
    /// The resource ends before its flags and signed frame count.
    ResourceHeaderTooShort {
        /// Number of available resource bytes.
        actual: usize,
    },
    /// The selected directory entry is not fully present.
    FrameTableEntryOutsideResource {
        /// Requested zero-based frame index.
        frame_index: usize,
        /// Minimum byte count needed to read its directory entry.
        required: usize,
        /// Number of available resource bytes.
        actual: usize,
    },
    /// A packed frame offset does not identify a complete frame header.
    FrameHeaderOutsideResource {
        /// Decoded flat byte offset.
        byte_offset: usize,
        /// Number of available resource bytes.
        actual: usize,
    },
}

impl fmt::Display for BridgeSpriteActivationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for BridgeSpriteActivationError {}

impl From<BridgeSpriteEntityError> for BridgeSpriteActivationError {
    fn from(error: BridgeSpriteEntityError) -> Self {
        Self::Entity(error)
    }
}

/// Activate one entity from a resource already retained by the flat cache.
///
/// This translates `entity_object_populate` at BLOODPRG routine offset
/// `0x0040D0`. A catalog identifier and immutable cache lookup replace the
/// resource-handle table, loaded bit, resolver register result, and far pointer.
pub fn populate_bridge_sprite_from_cache(
    cache: &OriginalResourceCache,
    entities: &mut [BridgeSpriteEntity],
    entity_index: usize,
    resource: ResourceId,
    draw_position: BridgeSpritePosition,
    frame_index: usize,
) -> Result<bool, BridgeSpriteActivationError> {
    populate_bridge_sprite_from_resolved_resource(
        entities,
        entity_index,
        resource,
        cache.resolve(resource),
        draw_position,
        frame_index,
    )
}

/// Activate one entity from a decoded flat sprite resource.
///
/// This translates `entity_record_setter` at BLOODPRG routine offset
/// `0x00414E`. The packed on-disk frame offset is decoded once into a checked
/// byte offset; native segment arithmetic, wrapped indices, and ambient
/// direction-flag behavior are deliberately absent from the runtime model.
pub fn activate_bridge_sprite_from_resource(
    entities: &mut [BridgeSpriteEntity],
    entity_index: usize,
    resource: ResourceId,
    resource_bytes: &[u8],
    draw_position: BridgeSpritePosition,
    frame_index: usize,
) -> Result<bool, BridgeSpriteActivationError> {
    populate_bridge_sprite_from_resolved_resource(
        entities,
        entity_index,
        resource,
        Some(resource_bytes),
        draw_position,
        frame_index,
    )
}

fn populate_bridge_sprite_from_resolved_resource(
    entities: &mut [BridgeSpriteEntity],
    entity_index: usize,
    resource: ResourceId,
    resource_bytes: Option<&[u8]>,
    draw_position: BridgeSpritePosition,
    frame_index: usize,
) -> Result<bool, BridgeSpriteActivationError> {
    let entity_count = entities.len();
    let entity = entities
        .get_mut(entity_index)
        .ok_or(BridgeSpriteEntityError {
            entity_index,
            entity_count,
        })?;
    let Some(resource_bytes) = resource_bytes else {
        return Ok(false);
    };
    if resource_bytes.len() < RESOURCE_HEADER_BYTE_COUNT {
        return Err(BridgeSpriteActivationError::ResourceHeaderTooShort {
            actual: resource_bytes.len(),
        });
    }

    let resource_flags = read_u16(resource_bytes, 0);
    let frame_count = read_u16(resource_bytes, size_of::<u16>()) as i16;
    if frame_count <= 0 || frame_index >= frame_count as usize {
        return Ok(false);
    }

    let table_offset = frame_index
        .checked_mul(RESOURCE_FRAME_TABLE_ENTRY_BYTE_COUNT)
        .and_then(|offset| RESOURCE_HEADER_BYTE_COUNT.checked_add(offset))
        .ok_or(
            BridgeSpriteActivationError::FrameTableEntryOutsideResource {
                frame_index,
                required: usize::MAX,
                actual: resource_bytes.len(),
            },
        )?;
    let table_end = table_offset
        .checked_add(RESOURCE_FRAME_TABLE_ENTRY_BYTE_COUNT)
        .ok_or(
            BridgeSpriteActivationError::FrameTableEntryOutsideResource {
                frame_index,
                required: usize::MAX,
                actual: resource_bytes.len(),
            },
        )?;
    if table_end > resource_bytes.len() {
        return Err(
            BridgeSpriteActivationError::FrameTableEntryOutsideResource {
                frame_index,
                required: table_end,
                actual: resource_bytes.len(),
            },
        );
    }

    let packed_frame_offset = read_u32(resource_bytes, table_offset) as usize;
    let frame_byte_offset = RESOURCE_HEADER_BYTE_COUNT
        .checked_add(packed_frame_offset)
        .ok_or(BridgeSpriteActivationError::FrameHeaderOutsideResource {
            byte_offset: usize::MAX,
            actual: resource_bytes.len(),
        })?;
    let frame_header_end = frame_byte_offset
        .checked_add(SPRITE_FRAME_HEADER_BYTE_COUNT)
        .ok_or(BridgeSpriteActivationError::FrameHeaderOutsideResource {
            byte_offset: frame_byte_offset,
            actual: resource_bytes.len(),
        })?;
    if frame_header_end > resource_bytes.len() {
        return Err(BridgeSpriteActivationError::FrameHeaderOutsideResource {
            byte_offset: frame_byte_offset,
            actual: resource_bytes.len(),
        });
    }

    let extent = BridgeSpriteExtent {
        width: read_u16(resource_bytes, frame_byte_offset),
        height: read_u16(resource_bytes, frame_byte_offset + size_of::<u16>()),
    };
    entity.flags = BridgeSpriteFlags::from_bits(
        resource_flags & RESOURCE_FRAME_ENCODING_FLAG | ACTIVATED_FLAGS,
    );
    entity.frame = Some(BridgeSpriteFrameReference {
        resource,
        frame_index,
        byte_offset: frame_byte_offset,
    });
    entity.source_extent = extent;
    entity.extent = extent;
    if entity.committed_extent.width == u16::MIN {
        entity.committed_extent.width = extent.width;
    }
    if entity.committed_extent.height == u16::MIN {
        entity.committed_extent.height = extent.height;
    }
    entity.draw_position = draw_position;
    Ok(true)
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Observable result of one sprite geometry update.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BridgeSpriteGeometryUpdate {
    /// The entity's state allowed geometry processing.
    pub accepted: bool,
    /// At least one draw coordinate changed.
    pub position_changed: bool,
    /// The stored destination extent changed.
    pub extent_changed: bool,
    /// A prior scaled-extent state returned to the comparison extent.
    pub scaled_extent_cleared: bool,
}

/// Advance one active bridge sprite from state zero to its dirty state.
///
/// This translates `entity_flag_state_transition` at BLOODPRG routine offset
/// `0x0041D1`. All unrelated low and high flag bits are retained while checked
/// flat indexing replaces the original fixed `GS` table address.
pub fn advance_bridge_sprite_state(
    entities: &mut [BridgeSpriteEntity],
    entity_index: usize,
) -> Result<bool, BridgeSpriteEntityError> {
    let entity_count = entities.len();
    let entity = entities
        .get_mut(entity_index)
        .ok_or(BridgeSpriteEntityError {
            entity_index,
            entity_count,
        })?;
    Ok(entity.flags.advance_state_zero())
}

/// Invalid bridge sprite entity selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeSpriteEntityError {
    /// Requested entity index.
    pub entity_index: usize,
    /// Available entity count.
    pub entity_count: usize,
}

impl fmt::Display for BridgeSpriteEntityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "bridge sprite entity {} is outside {} records",
            self.entity_index, self.entity_count
        )
    }
}

impl Error for BridgeSpriteEntityError {}

/// Invalid inclusive range into the bridge sprite entity table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeSpriteRangeError {
    /// First requested entity index.
    pub first_entity_index: usize,
    /// Last requested entity index, inclusive.
    pub last_entity_index: usize,
    /// Available entity count.
    pub entity_count: usize,
}

impl fmt::Display for BridgeSpriteRangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "bridge sprite range {}..={} is outside {} records",
            self.first_entity_index, self.last_entity_index, self.entity_count
        )
    }
}

impl Error for BridgeSpriteRangeError {}

/// Transition every active entity in an inclusive range to its dirty state.
///
/// This translates `sprite_slot_range_mark_dirty` at BLOODPRG routine offset
/// `0x004240`. The exact low-byte transition clears active and state-zero,
/// sets dirty, and retains every unrelated low and high flag bit. Checked slice
/// bounds replace the original wrapping 16-bit record and range arithmetic.
pub fn mark_bridge_sprite_range_dirty(
    entities: &mut [BridgeSpriteEntity],
    first_entity_index: usize,
    last_entity_index: usize,
) -> Result<usize, BridgeSpriteRangeError> {
    if first_entity_index > last_entity_index || last_entity_index >= entities.len() {
        return Err(BridgeSpriteRangeError {
            first_entity_index,
            last_entity_index,
            entity_count: entities.len(),
        });
    }

    Ok(entities[first_entity_index..=last_entity_index]
        .iter_mut()
        .fold(0, |changed, entity| {
            changed + usize::from(entity.flags.transition_active_slot_to_dirty())
        }))
}

/// Update one bridge sprite's logical draw position.
///
/// This translates `sprite_slot_position_update` at BLOODPRG routine offset
/// `0x00420D`. The entity index is resolved through a checked slice and each
/// coordinate independently marks the sprite dirty only when it changes.
pub fn update_bridge_sprite_position(
    entities: &mut [BridgeSpriteEntity],
    entity_index: usize,
    requested: BridgeSpritePosition,
) -> Result<BridgeSpriteGeometryUpdate, BridgeSpriteEntityError> {
    let entity_count = entities.len();
    let entity = entities
        .get_mut(entity_index)
        .ok_or(BridgeSpriteEntityError {
            entity_index,
            entity_count,
        })?;
    if !entity.flags.accepts_geometry_updates() {
        return Ok(BridgeSpriteGeometryUpdate::default());
    }

    let mut update = BridgeSpriteGeometryUpdate {
        accepted: true,
        ..BridgeSpriteGeometryUpdate::default()
    };
    if entity.draw_position.x != requested.x {
        entity.draw_position.x = requested.x;
        entity.flags.mark_dirty();
        update.position_changed = true;
    }
    if entity.draw_position.y != requested.y {
        entity.draw_position.y = requested.y;
        entity.flags.mark_dirty();
        update.position_changed = true;
    }
    Ok(update)
}

/// Update one bridge sprite's logical destination extent.
///
/// This translates `sprite_slot_extent_update` at BLOODPRG routine offset
/// `0x0042CD`. Its formerly implicit comparison input is an ordinary typed
/// extent. Matching it clears the scaled-extent flag; a changed custom extent
/// stores both dimensions and marks the entity dirty.
pub fn update_bridge_sprite_extent(
    entities: &mut [BridgeSpriteEntity],
    entity_index: usize,
    requested: BridgeSpriteExtent,
    comparison: BridgeSpriteExtent,
) -> Result<BridgeSpriteGeometryUpdate, BridgeSpriteEntityError> {
    let entity_count = entities.len();
    let entity = entities
        .get_mut(entity_index)
        .ok_or(BridgeSpriteEntityError {
            entity_index,
            entity_count,
        })?;
    if !entity.flags.accepts_geometry_updates() {
        return Ok(BridgeSpriteGeometryUpdate::default());
    }

    let mut update = BridgeSpriteGeometryUpdate {
        accepted: true,
        ..BridgeSpriteGeometryUpdate::default()
    };
    if requested == comparison {
        update.scaled_extent_cleared = entity.flags.clear_scaled_extent();
        return Ok(update);
    }
    if entity.extent != requested {
        entity.extent = requested;
        entity.flags.mark_scaled_extent();
        update.extent_changed = true;
    }
    Ok(update)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const POSITION_ORACLE_COUNT: usize = 6;
    const EXTENT_ORACLE_COUNT: usize = 8;
    const STATE_TRANSITION_ORACLE_COUNT: usize = 7;
    const ENTITY_POPULATE_ORACLE_COUNT: usize = 14;
    const ENTITY_ACTIVATION_ORACLE_COUNT: usize = 15;
    const RANGE_DIRTY_ORACLE_COUNT: usize = 4;
    const COORDINATE_COUNT: usize = 2;
    const MAXIMUM_SYNTHETIC_RESOURCE_BYTE_COUNT: usize = 1_000_000;
    const DIRECT_RESOURCE_ID: ResourceId = ResourceId::new(63);

    #[derive(Deserialize)]
    struct PositionOracle {
        name: String,
        object_id: usize,
        input_flags: u16,
        input_position: [u16; COORDINATE_COUNT],
        requested_position: [u16; COORDINATE_COUNT],
        output_flags: u16,
        output_position: [u16; COORDINATE_COUNT],
    }

    #[derive(Deserialize)]
    struct ExtentOracle {
        name: String,
        object_id: usize,
        input_flags: u16,
        requested_extent: [u16; COORDINATE_COUNT],
        stored_extent: [u16; COORDINATE_COUNT],
        source_extent: [u16; COORDINATE_COUNT],
        output_flags: u16,
        output_extent: [u16; COORDINATE_COUNT],
    }

    #[derive(Deserialize)]
    struct StateTransitionOracle {
        name: String,
        object_id: usize,
        input_flags: u16,
        output_flags: u16,
    }

    #[derive(Deserialize)]
    struct EntityPopulateOracle {
        name: String,
        entity_id: usize,
        resource_handle: u16,
        loaded: bool,
        resource_flags: u16,
        frame_count: u16,
        frame_index: usize,
        packed_frame: u32,
        accepted: bool,
        extent_source: [u16; COORDINATE_COUNT],
        committed_extents_before: [u16; COORDINATE_COUNT],
        draw_position: [u16; COORDINATE_COUNT],
    }

    #[derive(Deserialize)]
    struct EntityActivationOracle {
        name: String,
        entity_id: usize,
        resource_flags: u16,
        frame_count: u16,
        frame_index: usize,
        packed_frame: u32,
        accepted: bool,
        extent_source: [u16; COORDINATE_COUNT],
        committed_extents_before: [u16; COORDINATE_COUNT],
        draw_position: [u16; COORDINATE_COUNT],
    }

    #[derive(Deserialize)]
    struct RangeDirtyOracle {
        name: String,
        first_object_id: usize,
        last_object_id: usize,
        input_flags: Vec<u16>,
        output_flags: Vec<u16>,
    }

    struct ActivationCase<'a> {
        name: &'a str,
        entity_index: usize,
        resource: ResourceId,
        loaded: bool,
        resource_flags: u16,
        frame_count: u16,
        frame_index: usize,
        packed_frame: u32,
        native_accepted: bool,
        extent: BridgeSpriteExtent,
        committed_extent_before: BridgeSpriteExtent,
        draw_position: BridgeSpritePosition,
    }

    #[test]
    fn entity_population_matches_valid_original_vectors_and_rejects_native_aliases() {
        let vectors: Vec<EntityPopulateOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_40d0_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ENTITY_POPULATE_ORACLE_COUNT);

        for vector in vectors {
            run_activation_case(ActivationCase {
                name: &vector.name,
                entity_index: vector.entity_id,
                resource: ResourceId::new(vector.resource_handle),
                loaded: vector.loaded,
                resource_flags: vector.resource_flags,
                frame_count: vector.frame_count,
                frame_index: vector.frame_index,
                packed_frame: vector.packed_frame,
                native_accepted: vector.accepted,
                extent: extent(vector.extent_source),
                committed_extent_before: extent(vector.committed_extents_before),
                draw_position: position(vector.draw_position),
            });
        }
    }

    #[test]
    fn direct_entity_activation_matches_valid_original_vectors_and_rejects_native_aliases() {
        let vectors: Vec<EntityActivationOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_414e_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ENTITY_ACTIVATION_ORACLE_COUNT);

        for vector in vectors {
            run_activation_case(ActivationCase {
                name: &vector.name,
                entity_index: vector.entity_id,
                resource: DIRECT_RESOURCE_ID,
                loaded: true,
                resource_flags: vector.resource_flags,
                frame_count: vector.frame_count,
                frame_index: vector.frame_index,
                packed_frame: vector.packed_frame,
                native_accepted: vector.accepted,
                extent: extent(vector.extent_source),
                committed_extent_before: extent(vector.committed_extents_before),
                draw_position: position(vector.draw_position),
            });
        }
    }

    #[test]
    fn empty_cache_retains_the_original_unloaded_no_op() {
        let cache = OriginalResourceCache::new();
        let mut entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
        let before = entities;
        assert!(
            !populate_bridge_sprite_from_cache(
                &cache,
                &mut entities,
                1,
                ResourceId::new(2),
                BridgeSpritePosition { x: 10, y: 20 },
                0,
            )
            .unwrap()
        );
        assert_eq!(entities, before);
    }

    #[test]
    fn range_dirty_transition_matches_every_original_vector() {
        let vectors: Vec<RangeDirtyOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_4240_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), RANGE_DIRTY_ORACLE_COUNT);

        for vector in vectors {
            let mut entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
            for (entity, flags) in entities[vector.first_object_id..=vector.last_object_id]
                .iter_mut()
                .zip(vector.input_flags.iter().copied())
            {
                entity.flags = BridgeSpriteFlags::from_bits(flags);
            }

            let changed = mark_bridge_sprite_range_dirty(
                &mut entities,
                vector.first_object_id,
                vector.last_object_id,
            )
            .unwrap();
            let actual: Vec<u16> = entities[vector.first_object_id..=vector.last_object_id]
                .iter()
                .map(|entity| entity.flags.bits())
                .collect();
            assert_eq!(actual, vector.output_flags, "{}", vector.name);
            assert_eq!(
                changed,
                vector
                    .input_flags
                    .iter()
                    .filter(|flags| **flags & VISIBLE_FLAG != u16::MIN)
                    .count(),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn state_transition_matches_every_original_vector() {
        let vectors: Vec<StateTransitionOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_41d1_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), STATE_TRANSITION_ORACLE_COUNT);

        for vector in vectors {
            let mut entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
            entities[vector.object_id].flags = BridgeSpriteFlags::from_bits(vector.input_flags);

            let changed = advance_bridge_sprite_state(&mut entities, vector.object_id).unwrap();

            assert_eq!(
                entities[vector.object_id].flags.bits(),
                vector.output_flags,
                "{}",
                vector.name
            );
            assert_eq!(
                changed,
                vector.input_flags != vector.output_flags,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn position_updates_match_every_original_vector() {
        let vectors: Vec<PositionOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_420d_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), POSITION_ORACLE_COUNT);

        for vector in vectors {
            let mut entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
            entities[vector.object_id].flags = BridgeSpriteFlags::from_bits(vector.input_flags);
            entities[vector.object_id].draw_position = position(vector.input_position);
            update_bridge_sprite_position(
                &mut entities,
                vector.object_id,
                position(vector.requested_position),
            )
            .unwrap();
            assert_eq!(
                entities[vector.object_id].flags.bits(),
                vector.output_flags,
                "{}",
                vector.name
            );
            assert_eq!(
                entities[vector.object_id].draw_position,
                position(vector.output_position),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn extent_updates_match_every_original_vector() {
        let vectors: Vec<ExtentOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_42cd_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), EXTENT_ORACLE_COUNT);

        for vector in vectors {
            let mut entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
            entities[vector.object_id].flags = BridgeSpriteFlags::from_bits(vector.input_flags);
            entities[vector.object_id].extent = extent(vector.stored_extent);
            update_bridge_sprite_extent(
                &mut entities,
                vector.object_id,
                extent(vector.requested_extent),
                extent(vector.source_extent),
            )
            .unwrap();
            assert_eq!(
                entities[vector.object_id].flags.bits(),
                vector.output_flags,
                "{}",
                vector.name
            );
            assert_eq!(
                entities[vector.object_id].extent,
                extent(vector.output_extent),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn invalid_entity_indices_do_not_mutate_the_table() {
        let mut entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
        let before = entities;
        assert_eq!(
            update_bridge_sprite_position(
                &mut entities,
                BRIDGE_SPRITE_ENTITY_COUNT,
                BridgeSpritePosition::default(),
            ),
            Err(BridgeSpriteEntityError {
                entity_index: BRIDGE_SPRITE_ENTITY_COUNT,
                entity_count: BRIDGE_SPRITE_ENTITY_COUNT,
            })
        );
        assert_eq!(entities, before);
    }

    #[test]
    fn invalid_dirty_ranges_do_not_mutate_the_table() {
        let mut entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
        let before = entities;
        assert_eq!(
            mark_bridge_sprite_range_dirty(&mut entities, 4, 3),
            Err(BridgeSpriteRangeError {
                first_entity_index: 4,
                last_entity_index: 3,
                entity_count: BRIDGE_SPRITE_ENTITY_COUNT,
            })
        );
        assert_eq!(entities, before);
    }

    fn run_activation_case(case: ActivationCase<'_>) {
        let resource_bytes = synthetic_sprite_resource(&case);
        let mut entities = [BridgeSpriteEntity::default(); BRIDGE_SPRITE_ENTITY_COUNT];
        if case.entity_index < entities.len() {
            entities[case.entity_index].committed_extent = case.committed_extent_before;
        }
        let before = entities;
        let result = populate_bridge_sprite_from_resolved_resource(
            &mut entities,
            case.entity_index,
            case.resource,
            case.loaded.then_some(resource_bytes.as_slice()),
            case.draw_position,
            case.frame_index,
        );

        if case.entity_index >= BRIDGE_SPRITE_ENTITY_COUNT {
            assert!(
                matches!(result, Err(BridgeSpriteActivationError::Entity(_))),
                "{}",
                case.name
            );
            assert_eq!(entities, before, "{}", case.name);
            return;
        }
        if !case.loaded || !case.native_accepted {
            assert_eq!(result, Ok(false), "{}", case.name);
            assert_eq!(entities, before, "{}", case.name);
            return;
        }
        if case.frame_index > i16::MAX as usize {
            assert_eq!(result, Ok(false), "{}", case.name);
            assert_eq!(entities, before, "{}", case.name);
            return;
        }
        if case.packed_frame as usize > MAXIMUM_SYNTHETIC_RESOURCE_BYTE_COUNT {
            assert!(
                matches!(
                    result,
                    Err(BridgeSpriteActivationError::FrameHeaderOutsideResource { .. })
                ),
                "{}",
                case.name
            );
            assert_eq!(entities, before, "{}", case.name);
            return;
        }

        assert_eq!(result, Ok(true), "{}", case.name);
        let entity = entities[case.entity_index];
        let frame_byte_offset = RESOURCE_HEADER_BYTE_COUNT + case.packed_frame as usize;
        assert_eq!(
            entity.flags.bits(),
            case.resource_flags & RESOURCE_FRAME_ENCODING_FLAG | ACTIVATED_FLAGS,
            "{}",
            case.name
        );
        assert_eq!(
            entity.frame,
            Some(BridgeSpriteFrameReference {
                resource: case.resource,
                frame_index: case.frame_index,
                byte_offset: frame_byte_offset,
            }),
            "{}",
            case.name
        );
        assert_eq!(entity.source_extent, case.extent, "{}", case.name);
        assert_eq!(entity.extent, case.extent, "{}", case.name);
        assert_eq!(entity.draw_position, case.draw_position, "{}", case.name);
        assert_eq!(
            entity.committed_extent,
            BridgeSpriteExtent {
                width: if case.committed_extent_before.width == u16::MIN {
                    case.extent.width
                } else {
                    case.committed_extent_before.width
                },
                height: if case.committed_extent_before.height == u16::MIN {
                    case.extent.height
                } else {
                    case.committed_extent_before.height
                },
            },
            "{}",
            case.name
        );
    }

    fn synthetic_sprite_resource(case: &ActivationCase<'_>) -> Vec<u8> {
        let signed_frame_count = case.frame_count as i16;
        let selected_frame_is_valid = signed_frame_count > 0
            && case.frame_index < signed_frame_count as usize
            && case.frame_index <= i16::MAX as usize;
        let table_end = if selected_frame_is_valid {
            RESOURCE_HEADER_BYTE_COUNT
                + (case.frame_index + 1) * RESOURCE_FRAME_TABLE_ENTRY_BYTE_COUNT
        } else {
            RESOURCE_HEADER_BYTE_COUNT
        };
        let frame_byte_offset = RESOURCE_HEADER_BYTE_COUNT + case.packed_frame as usize;
        let frame_end = if selected_frame_is_valid
            && case.packed_frame as usize <= MAXIMUM_SYNTHETIC_RESOURCE_BYTE_COUNT
        {
            frame_byte_offset + SPRITE_FRAME_HEADER_BYTE_COUNT
        } else {
            RESOURCE_HEADER_BYTE_COUNT
        };
        let mut bytes = vec![u8::MIN; table_end.max(frame_end)];
        bytes[..size_of::<u16>()].copy_from_slice(&case.resource_flags.to_le_bytes());
        bytes[size_of::<u16>()..RESOURCE_HEADER_BYTE_COUNT]
            .copy_from_slice(&case.frame_count.to_le_bytes());
        if selected_frame_is_valid {
            let table_offset = RESOURCE_HEADER_BYTE_COUNT
                + case.frame_index * RESOURCE_FRAME_TABLE_ENTRY_BYTE_COUNT;
            bytes[table_offset..table_offset + RESOURCE_FRAME_TABLE_ENTRY_BYTE_COUNT]
                .copy_from_slice(&case.packed_frame.to_le_bytes());
            if frame_end > RESOURCE_HEADER_BYTE_COUNT {
                bytes[frame_byte_offset..frame_byte_offset + size_of::<u16>()]
                    .copy_from_slice(&case.extent.width.to_le_bytes());
                bytes[frame_byte_offset + size_of::<u16>()
                    ..frame_byte_offset + size_of::<u16>() * COORDINATE_COUNT]
                    .copy_from_slice(&case.extent.height.to_le_bytes());
            }
        }
        bytes
    }

    fn position(words: [u16; COORDINATE_COUNT]) -> BridgeSpritePosition {
        BridgeSpritePosition {
            x: words[0],
            y: words[1],
        }
    }

    fn extent(words: [u16; COORDINATE_COUNT]) -> BridgeSpriteExtent {
        BridgeSpriteExtent {
            width: words[0],
            height: words[1],
        }
    }
}
