#include "../include/xdb_manu3.h"

void XDB_FAR xdb_manu3_api_entry(
        const volatile xdb_manu3_api_request XDB_FAR *request,
        xdb_u16 code_segment)
{
    xdb_u16 selector;
    xdb_u16 saved_pitch;
    xdb_u16 saved_yaw;
    xdb_u16 cursor_delta;
    xdb_u16 geometry_segment;
    const volatile xdb_manu3_projection_state XDB_NEAR *state;
    const volatile xdb_manu3_point3 XDB_FAR *reference;
    xdb_i32 object_x;
    xdb_i32 object_y;
    xdb_i32 object_z;
    xdb_u32 accumulator;
    xdb_i32 depth;

    if (xdb_manu3_data_segment == 0u) {
        xdb_manu3_init_protocol(code_segment);
        return;
    }

    xdb_manu3_cursor = request->cursor;
    xdb_manu3_framebuffer_segment = (xdb_u16)(
            0xa000u + (request->framebuffer_window_offset >> 4));

    selector = request->animation_selector & 0x001fu;
    if (selector != 0u) {
        xdb_manu3_anim_select(selector);
    }
    xdb_manu3_tween_step();

    saved_pitch = xdb_manu3_view_pitch;
    saved_yaw = xdb_manu3_view_yaw;
    cursor_delta = (xdb_u16)(
            ((xdb_u16)xdb_manu3_cursor.x - 0x00a0u) << 1);
    xdb_manu3_view_yaw = (xdb_u16)(
            xdb_manu3_view_yaw + cursor_delta);
    cursor_delta = (xdb_u16)(
            ((xdb_u16)xdb_manu3_cursor.y - 0x0064u) << 1);
    xdb_manu3_view_pitch = (xdb_u16)(
            xdb_manu3_view_pitch + cursor_delta);
    xdb_manu3_matrix_build();
    xdb_manu3_view_yaw = saved_yaw;
    xdb_manu3_view_pitch = saved_pitch;

    geometry_segment = xdb_manu3_segments.work_segment_0;
    state = &xdb_manu3_projection_states[3];
    reference = XDB_FAR_AT(
            const volatile xdb_manu3_point3,
            geometry_segment,
            0x02acu);
    object_x = reference->x;
    object_y = reference->y;
    object_z = reference->z;

    accumulator = (xdb_u32)state->matrix[2][0] * (xdb_u32)object_x;
    accumulator += (xdb_u32)state->matrix[2][1] * (xdb_u32)object_y;
    accumulator += (xdb_u32)state->matrix[2][2] * (xdb_u32)object_z;
    accumulator += (xdb_u32)state->translation[2];
    depth = (xdb_i32)accumulator >> 8;

    if (depth > 0) {
        xdb_u32 screen_accumulator;
        xdb_i32 screen_offset;

        screen_accumulator = (xdb_u32)state->matrix[1][0]
                * (xdb_u32)object_x;
        screen_accumulator += (xdb_u32)state->matrix[1][1]
                * (xdb_u32)object_y;
        screen_accumulator += (xdb_u32)state->matrix[1][2]
                * (xdb_u32)object_z;
        screen_accumulator += (xdb_u32)state->translation[1];
        screen_offset = (xdb_i32)screen_accumulator / depth;
        xdb_manu3_screen_center_y = (xdb_i32)(
                (xdb_u32)(xdb_i32)xdb_manu3_cursor.y
                + (xdb_u32)screen_offset);

        screen_accumulator = (xdb_u32)state->matrix[0][0]
                * (xdb_u32)object_x;
        screen_accumulator += (xdb_u32)state->matrix[0][1]
                * (xdb_u32)object_y;
        screen_accumulator += (xdb_u32)state->matrix[0][2]
                * (xdb_u32)object_z;
        screen_accumulator += (xdb_u32)state->translation[0];
        screen_offset = (xdb_i32)screen_accumulator / depth;
        xdb_manu3_screen_center_x = (xdb_i32)(
                (xdb_u32)(xdb_i32)xdb_manu3_cursor.x
                - (xdb_u32)screen_offset);
    }

    xdb_manu3_entity_project();
    xdb_manu3_face_builder_next();
}
