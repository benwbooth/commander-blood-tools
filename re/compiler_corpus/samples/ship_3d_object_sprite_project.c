/* Codegen probe for BLOODPRG 0x009B98. */
#include <string.h>

typedef unsigned int u16;
typedef signed int i16;
typedef unsigned long u32;
typedef signed long i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define FAR
#define GAME_DATA FAR
#endif

typedef struct projection_context {
    i32 matrix[9];
    u16 projected_x;
    u16 projected_y;
    u16 projected_depth;
    u16 depth_scale;
} projection_context;

typedef struct point_record {
    u16 x;
    u16 y;
    u16 z;
    u16 scratch;
} point_record;

typedef struct object_anchor {
    u16 x;
    u16 y;
    u16 z;
} object_anchor;

typedef struct source_extent {
    u16 width;
    u16 height;
} source_extent;

typedef union extent_comparison_reference {
    i32 coefficient;
    const volatile source_extent FAR *extent;
} extent_comparison_reference;

typedef struct entity_record {
    u16 flags;
    u16 field_02;
    const volatile source_extent FAR *source;
    u16 draw_x;
    u16 draw_y;
    u16 extent_width;
    u16 extent_height;
    u16 tail[8];
} entity_record;

extern volatile projection_context GAME_DATA projection;
extern volatile point_record GAME_DATA projection_work;
extern volatile object_anchor GAME_DATA anchors[];
extern volatile entity_record GAME_DATA entities[];
extern volatile u16 GAME_DATA projection_remaining;
extern volatile i16 GAME_DATA camera_x;
extern volatile i16 GAME_DATA camera_y;
extern volatile i16 GAME_DATA camera_z;

void FAR extent_probe(u16 entity_id, u16 width, u16 height,
        const volatile source_extent FAR *source);
void FAR position_probe(u16 entity_id, u16 draw_x, u16 draw_y);

#if defined(__WATCOMC__)
#pragma aux extent_probe parm [ax] [cx] [dx] [es si]
#pragma aux position_probe parm [ax] [bx] [cx]
#pragma aux ship_3d_object_sprite_project_probe modify exact []
#endif

void FAR ship_3d_object_sprite_project_probe(void)
{
    volatile object_anchor GAME_DATA *anchor;
    volatile entity_record GAME_DATA *record;
    const volatile source_extent FAR *comparison_extent;
    const volatile source_extent FAR *source;
    u16 scaled_height;
    u16 scaled_width;
    u16 entity_id;
    i32 axis;
    i32 depth;
    i16 x;
    i16 y;
    i16 z;

    anchor = anchors;
    projection_remaining = 11u;

    while (projection_remaining-- != 0u) {
        projection_work = *(const volatile point_record GAME_DATA *)anchor;
        ++anchor;
        entity_id = (u16)(projection_remaining + 0x15u);
        record = &entities[entity_id];
        if ((record->flags & 0x80u) == 0u) {
            continue;
        }

        projection_work.x = (u16)(projection_work.x - (u16)camera_x);
        projection_work.y = (u16)(projection_work.y - (u16)camera_y);
        projection_work.z = (u16)(projection_work.z - (u16)camera_z);
        x = (i16)projection_work.x;
        y = (i16)projection_work.y;
        z = (i16)projection_work.z;
        depth = (((i32)x * projection.matrix[6])
                + ((i32)y * projection.matrix[7])
                + ((i32)z * projection.matrix[8])) >> 15;
        if (depth == 0L) {
            continue;
        }
        if (depth < 0L) {
            depth += 0x10000L;
        }

        projection.depth_scale = (u16)(0x100000UL / (u32)depth);
        axis = (((i32)x * projection.matrix[0])
                + ((i32)y * projection.matrix[1])
                + ((i32)z * projection.matrix[2])) >> 7;
        projection.projected_x = (u16)((axis / depth) + 160L);
        axis = (((i32)x * projection.matrix[3])
                + ((i32)y * projection.matrix[4])
                + ((i32)z * projection.matrix[5])) >> 7;
        projection.projected_y = (u16)((axis / depth) + 100L);
        projection.projected_depth = (u16)depth;

        source = record->source;
        scaled_width = (u16)(((u32)source->width * projection.depth_scale) >> 10);
        scaled_height = (u16)(((u32)source->height * projection.depth_scale) >> 10);
        comparison_extent =
                ((const volatile extent_comparison_reference GAME_DATA *)
                &projection.matrix[1])->extent;
        extent_probe(entity_id, scaled_width, scaled_height, comparison_extent);
        position_probe(entity_id,
                (u16)(projection.projected_x - (record->extent_width >> 1)),
                (u16)(projection.projected_y - (record->extent_height >> 1)));
    }
}
