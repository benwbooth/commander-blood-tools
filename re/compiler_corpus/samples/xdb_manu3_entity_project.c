/* Codegen probe for the MANU3 state/vertex projection stage. */
#include <dos.h>

typedef unsigned char u8;
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

typedef union vertex_field_004 {
    i16 object_x;
    u16 projection_source_offset;
} vertex_field_004;

typedef union screen_position {
    u32 packed;
    struct {
        i16 x;
        i16 y;
    } position;
} screen_position;

typedef struct projected_vertex {
    u16 link;
    u16 field_002;
    vertex_field_004 field_004;
    i16 object_y;
    i16 object_z;
    screen_position screen;
    i32 depth;
    u16 clip_flags;
} projected_vertex;

typedef struct projection_state {
    u16 field_000;
    u16 vertex_count;
    u16 field_004;
    u16 vertex_offset;
    u8 field_008[0x0A];
    i32 matrix[3][3];
    i32 translation[3];
    u8 field_042[0x1C];
} projection_state;

typedef struct segment_directory {
    u16 field_000;
    u16 work_segment_0;
} segment_directory;

extern volatile segment_directory segments;
extern volatile i32 screen_center_x;
extern volatile i32 screen_center_y;
extern volatile u16 projection_remaining;
extern volatile u16 projection_field_224e;
extern volatile u16 projection_state_count;
extern volatile u16 projection_copy_offset;
extern volatile u16 projection_copy_count;
extern volatile projection_state projection_states[];

void NEAR xdb_manu3_entity_project_probe(void);

#if defined(__WATCOMC__)
#pragma aux xdb_manu3_entity_project_probe \
        modify exact [ax bx cx dx si di bp]
#endif

void NEAR xdb_manu3_entity_project_probe(void)
{
    u16 geometry_segment = segments.work_segment_0;
    u16 state_count = projection_state_count;
    u16 copy_count;
    volatile projection_state NEAR *state = projection_states;

    do {
        u16 vertex_offset = state->vertex_offset;

        projection_remaining = state->vertex_count;
        projection_field_224e = 0;
        do {
            volatile projected_vertex FAR *vertex = FAR_AT(
                    volatile projected_vertex,
                    geometry_segment,
                    vertex_offset);
            i32 object_x = vertex->field_004.object_x;
            i32 object_y = vertex->object_y;
            i32 object_z = vertex->object_z;
            u32 accumulator;
            i32 depth;

            vertex->clip_flags = 0x8000u;
            accumulator = (u32)state->matrix[2][0] * (u32)object_x;
            accumulator += (u32)state->matrix[2][1] * (u32)object_y;
            accumulator += (u32)state->matrix[2][2] * (u32)object_z;
            accumulator += (u32)state->translation[2];
            depth = (i32)accumulator >> 8;
            vertex->depth = depth;

            if (depth > 0) {
                u32 screen_x_accumulator;
                u32 screen_y_accumulator;
                i32 screen_x;
                i32 screen_y;
                u16 clip_flags = 0;

                screen_y_accumulator = (u32)state->matrix[1][0]
                        * (u32)object_x;
                screen_y_accumulator += (u32)state->matrix[1][1]
                        * (u32)object_y;
                screen_y_accumulator += (u32)state->matrix[1][2]
                        * (u32)object_z;
                screen_y_accumulator += (u32)state->translation[1];
                screen_x_accumulator = (u32)state->matrix[0][0]
                        * (u32)object_x;
                screen_x_accumulator += (u32)state->matrix[0][1]
                        * (u32)object_y;
                screen_x_accumulator += (u32)state->matrix[0][2]
                        * (u32)object_z;
                screen_x_accumulator += (u32)state->translation[0];

                screen_x = (i32)screen_x_accumulator / depth;
                screen_y = (i32)(
                        0u - (u32)((i32)screen_y_accumulator / depth));
                screen_x = (i32)((u32)screen_x + (u32)screen_center_x);
                if (screen_x < 0) {
                    clip_flags = 0x0001u;
                    if (screen_x <= -40) {
                        screen_x = -39;
                    }
                }
                if (screen_x >= 320) {
                    clip_flags = 0x0002u;
                    if (screen_x >= 360) {
                        screen_x = 359;
                    }
                }

                screen_y = (i32)((u32)screen_y + (u32)screen_center_y);
                if (screen_y < 0) {
                    clip_flags |= 0x0004u;
                    if (screen_y <= -100) {
                        screen_y = -99;
                    }
                }
                if (screen_y >= 200) {
                    clip_flags |= 0x0008u;
                    if (screen_y >= 300) {
                        screen_y = 299;
                    }
                }

                vertex->clip_flags = clip_flags;
                vertex->screen.position.x = (i16)screen_x;
                vertex->screen.position.y = (i16)screen_y;
            }

            vertex_offset = (u16)(vertex_offset + sizeof(projected_vertex));
        } while (--projection_remaining != 0u);

        ++state;
    } while (--state_count != 0u);

    copy_count = projection_copy_count;
    if (copy_count != 0u) {
        u16 destination_offset = projection_copy_offset;

        do {
            volatile projected_vertex FAR *destination = FAR_AT(
                    volatile projected_vertex,
                    geometry_segment,
                    destination_offset);
            volatile projected_vertex FAR *source = FAR_AT(
                    volatile projected_vertex,
                    geometry_segment,
                    destination->field_004.projection_source_offset);
            u32 screen = source->screen.packed;
            i32 depth = source->depth;
            u16 clip_flags = source->clip_flags;

            destination->screen.packed = screen;
            destination->depth = depth;
            destination->clip_flags = clip_flags;
            destination_offset = (u16)(
                    destination_offset + sizeof(projected_vertex));
        } while (--copy_count != 0u);
    }
}
