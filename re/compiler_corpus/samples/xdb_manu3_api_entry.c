/* Codegen probe for the MANU3 cursor-driven far API coordinator. */
#include <dos.h>

typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;
typedef signed long i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#define FAR_AT(type, segment, offset) \
    ((type FAR *)MK_FP((segment), (offset)))

typedef struct cursor_position {
    i16 x;
    i16 y;
} cursor_position;

typedef struct api_request {
    cursor_position cursor;
    u16 animation_selector;
    u16 framebuffer_window_offset;
} api_request;

typedef struct point3 {
    i16 x;
    i16 y;
    i16 z;
} point3;

typedef struct projection_state {
    u16 field_000;
    u16 vertex_count;
    u16 field_004;
    u16 vertex_offset;
    unsigned char field_008[0x0A];
    i32 matrix[3][3];
    i32 translation[3];
    unsigned char field_042[0x1C];
} projection_state;

typedef struct segment_directory {
    u16 field_000;
    u16 work_segment_0;
} segment_directory;

extern volatile u16 active_data_segment;
extern volatile cursor_position cursor;
extern volatile u16 framebuffer_segment;
extern volatile u16 view_pitch;
extern volatile u16 view_yaw;
extern volatile i32 screen_center_x;
extern volatile i32 screen_center_y;
extern volatile segment_directory segments;
extern volatile projection_state projection_states[];

extern void FAR init_protocol_probe(u16 code_segment);
extern void NEAR anim_select_probe(u16 selector);
extern void NEAR tween_step_probe(void);
extern void NEAR matrix_build_probe(void);
extern void NEAR entity_project_probe(void);
extern void NEAR face_builder_next_probe(void);

void FAR xdb_manu3_api_entry_probe(
        const volatile api_request FAR *request,
        u16 code_segment)
{
    u16 selector;
    u16 saved_pitch;
    u16 saved_yaw;
    u16 cursor_delta;
    u16 geometry_segment;
    const volatile projection_state NEAR *state;
    const volatile point3 FAR *reference;
    i32 object_x;
    i32 object_y;
    i32 object_z;
    u32 accumulator;
    i32 depth;

    if (active_data_segment == 0u) {
        init_protocol_probe(code_segment);
        return;
    }

    cursor = request->cursor;
    framebuffer_segment = (u16)(
            0xa000u + (request->framebuffer_window_offset >> 4));

    selector = request->animation_selector & 0x001fu;
    if (selector != 0u) {
        anim_select_probe(selector);
    }
    tween_step_probe();

    saved_pitch = view_pitch;
    saved_yaw = view_yaw;
    cursor_delta = (u16)(((u16)cursor.x - 0x00a0u) << 1);
    view_yaw = (u16)(view_yaw + cursor_delta);
    cursor_delta = (u16)(((u16)cursor.y - 0x0064u) << 1);
    view_pitch = (u16)(view_pitch + cursor_delta);
    matrix_build_probe();
    view_yaw = saved_yaw;
    view_pitch = saved_pitch;

    geometry_segment = segments.work_segment_0;
    state = &projection_states[3];
    reference = FAR_AT(const volatile point3, geometry_segment, 0x02acu);
    object_x = reference->x;
    object_y = reference->y;
    object_z = reference->z;

    accumulator = (u32)state->matrix[2][0] * (u32)object_x;
    accumulator += (u32)state->matrix[2][1] * (u32)object_y;
    accumulator += (u32)state->matrix[2][2] * (u32)object_z;
    accumulator += (u32)state->translation[2];
    depth = (i32)accumulator >> 8;

    if (depth > 0) {
        u32 screen_accumulator;
        i32 screen_offset;

        screen_accumulator = (u32)state->matrix[1][0] * (u32)object_x;
        screen_accumulator += (u32)state->matrix[1][1] * (u32)object_y;
        screen_accumulator += (u32)state->matrix[1][2] * (u32)object_z;
        screen_accumulator += (u32)state->translation[1];
        screen_offset = (i32)screen_accumulator / depth;
        screen_center_y = (i32)(
                (u32)(i32)cursor.y + (u32)screen_offset);

        screen_accumulator = (u32)state->matrix[0][0] * (u32)object_x;
        screen_accumulator += (u32)state->matrix[0][1] * (u32)object_y;
        screen_accumulator += (u32)state->matrix[0][2] * (u32)object_z;
        screen_accumulator += (u32)state->translation[0];
        screen_offset = (i32)screen_accumulator / depth;
        screen_center_x = (i32)(
                (u32)(i32)cursor.x - (u32)screen_offset);
    }

    entity_project_probe();
    face_builder_next_probe();
}
