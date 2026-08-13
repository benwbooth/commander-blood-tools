#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_ship3d.h"

#define SHIP_3D_MATRIX_SHIFT 15
#define SHIP_3D_AXIS_SHIFT 7
#define SHIP_3D_DIMENSION_SHIFT 10
#define SHIP_3D_SCREEN_CENTER_X 160L
#define SHIP_3D_SCREEN_CENTER_Y 100L
#define SHIP_3D_DEPTH_WRAP 0x10000L
#define SHIP_3D_SCALE_NUMERATOR 0x100000UL

typedef union ship_3d_extent_comparison_reference {
    cb_i32 coefficient;
    const volatile bloodprg_sprite_source_extent CB_FAR *extent;
} ship_3d_extent_comparison_reference;

typedef char ship_3d_extent_comparison_reference_size_must_be_4[
        sizeof(ship_3d_extent_comparison_reference) == 4 ? 1 : -1];

void CB_FAR ship_3d_object_sprite_project(void)
{
    volatile ship_3d_object_anchor CB_GAME_DATA *anchor;
    volatile bloodprg_entity_record CB_GAME_DATA *record;
    const volatile bloodprg_sprite_source_extent CB_FAR *comparison_extent;
    const volatile bloodprg_sprite_source_extent CB_FAR *source_extent;
    cb_u16 scaled_height;
    cb_u16 scaled_width;
    cb_u16 entity_id;
    cb_i32 axis;
    cb_i32 depth;
    cb_i16 x;
    cb_i16 y;
    cb_i16 z;

    anchor = ship_3d_object_anchors;
    ship_3d_projection_remaining = SHIP_3D_OBJECT_ANCHOR_COUNT;

    while (ship_3d_projection_remaining-- != 0u) {
        ship_3d_projection_work =
                *(const volatile ship_3d_point_record CB_GAME_DATA *)anchor;
        ++anchor;
        entity_id = (cb_u16)(
                ship_3d_projection_remaining + SHIP_3D_NAV_ENTITY_BASE);
        record = &bloodprg_entity_table[entity_id];

        if ((record->flags & BLOODPRG_ENTITY_ACTIVE_FLAG) == 0u) {
            continue;
        }

        ship_3d_projection_work.x = (cb_u16)(
                ship_3d_projection_work.x - (cb_u16)ship_3d_camera_x);
        ship_3d_projection_work.y = (cb_u16)(
                ship_3d_projection_work.y - (cb_u16)ship_3d_camera_y);
        ship_3d_projection_work.z = (cb_u16)(
                ship_3d_projection_work.z - (cb_u16)ship_3d_camera_z);

        x = (cb_i16)ship_3d_projection_work.x;
        y = (cb_i16)ship_3d_projection_work.y;
        z = (cb_i16)ship_3d_projection_work.z;
        depth = (((cb_i32)x * ship_3d_projection.matrix[6])
                + ((cb_i32)y * ship_3d_projection.matrix[7])
                + ((cb_i32)z * ship_3d_projection.matrix[8]))
                >> SHIP_3D_MATRIX_SHIFT;
        if (depth == 0L) {
            continue;
        }
        if (depth < 0L) {
            depth += SHIP_3D_DEPTH_WRAP;
        }

        ship_3d_projection.depth_scale = (cb_u16)(
                SHIP_3D_SCALE_NUMERATOR / (cb_u32)depth);
        axis = (((cb_i32)x * ship_3d_projection.matrix[0])
                + ((cb_i32)y * ship_3d_projection.matrix[1])
                + ((cb_i32)z * ship_3d_projection.matrix[2]))
                >> SHIP_3D_AXIS_SHIFT;
        ship_3d_projection.projected_x = (cb_u16)(
                (axis / depth) + SHIP_3D_SCREEN_CENTER_X);

        axis = (((cb_i32)x * ship_3d_projection.matrix[3])
                + ((cb_i32)y * ship_3d_projection.matrix[4])
                + ((cb_i32)z * ship_3d_projection.matrix[5]))
                >> SHIP_3D_AXIS_SHIFT;
        ship_3d_projection.projected_y = (cb_u16)(
                (axis / depth) + SHIP_3D_SCREEN_CENTER_Y);
        ship_3d_projection.projected_depth = (cb_u16)depth;

        source_extent = (const volatile bloodprg_sprite_source_extent CB_FAR *)
                record->frame;
        scaled_width = (cb_u16)(
                ((cb_u32)source_extent->width *
                ship_3d_projection.depth_scale) >> SHIP_3D_DIMENSION_SHIFT);
        scaled_height = (cb_u16)(
                ((cb_u32)source_extent->height *
                ship_3d_projection.depth_scale) >> SHIP_3D_DIMENSION_SHIFT);
        comparison_extent =
                ((const volatile ship_3d_extent_comparison_reference
                CB_GAME_DATA *)&ship_3d_projection.matrix[1])->extent;
        sprite_slot_extent_update(entity_id,
                scaled_width, scaled_height, comparison_extent);
        sprite_slot_position_update(entity_id,
                (cb_u16)(ship_3d_projection.projected_x -
                        (record->extent_width >> 1)),
                (cb_u16)(ship_3d_projection.projected_y -
                        (record->extent_height >> 1)));
    }
}
