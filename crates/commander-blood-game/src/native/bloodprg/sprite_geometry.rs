//! Shared bridge sprite geometry updates over a typed entity table.

use std::error::Error;
use std::fmt;

/// Number of records in the recovered bridge sprite entity table.
pub const BRIDGE_SPRITE_ENTITY_COUNT: usize = 32;

const STATE_ZERO_FLAG: u16 = 1;
const DIRTY_FLAG: u16 = 2;
const EXTENT_CHANGED_FLAG: u16 = 16;
const VISIBLE_FLAG: u16 = 128;
const GEOMETRY_UPDATE_MASK: u16 = STATE_ZERO_FLAG | VISIBLE_FLAG;

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

/// Render-facing geometry owned by one bridge sprite entity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BridgeSpriteEntity {
    /// Recovered state, visibility, dirty, and extent flags.
    pub flags: BridgeSpriteFlags,
    /// Dimensions of the decoded source frame used for perspective scaling.
    pub source_extent: BridgeSpriteExtent,
    /// Current logical draw position.
    pub draw_position: BridgeSpritePosition,
    /// Current logical destination extent.
    pub extent: BridgeSpriteExtent,
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
    const COORDINATE_COUNT: usize = 2;

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
