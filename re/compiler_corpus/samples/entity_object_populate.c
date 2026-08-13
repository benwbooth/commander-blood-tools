#include <dos.h>

typedef unsigned char u8;
typedef signed int i16;
typedef unsigned int u16;
typedef unsigned long u32;

typedef struct resource_resolve_result {
    u16 segment;
    u16 offset;
    u16 loaded;
} resource_resolve_result;

typedef struct entity_resource {
    u16 flags;
    i16 frame_count;
    u32 packed_frame_offsets[1];
} entity_resource;

typedef struct sprite_frame {
    u16 stride;
    u16 height;
    i16 x_offset;
    i16 y_offset;
    u8 pixels[1];
} sprite_frame;

typedef struct entity_record {
    u16 flags;
    u16 field_02;
    const volatile sprite_frame far *frame;
    u16 draw_x;
    u16 draw_y;
    u16 extent_width;
    u16 extent_height;
    u16 committed_draw_x;
    u16 committed_draw_y;
    u16 committed_extent_width;
    u16 committed_extent_height;
    u16 dirty_rect[4];
} entity_record;

extern volatile entity_record entity_table[];
resource_resolve_result far resource_handle_resolve(u16 handle);

#define ENTITY_RESOURCE_FLAG 0x0004u
#define ENTITY_ACTIVATE_FLAGS 0x0083u

void far entity_object_populate(u16 entity_id,
        u16 resource_handle,
        u16 draw_x,
        u16 draw_y,
        u16 frame_index);
#pragma aux entity_object_populate \
        parm caller [ax] [dx] [bx] [cx] modify exact []

void far entity_object_populate(
        u16 entity_id,
        u16 resource_handle,
        u16 draw_x,
        u16 draw_y,
        u16 frame_index)
{
    resource_resolve_result resolved;
    const volatile entity_resource far *resource;
    const volatile sprite_frame far *frame;
    volatile entity_record far *record;
    u32 packed_frame;
    u16 selected_frame;
    u16 frame_segment;
    u16 frame_offset;

    _asm push ax;
    _asm push dx;
    _asm push es;
    _asm push ds;

    selected_frame = frame_index;
    _asm mov ax, word ptr [bp];
    _asm mov selected_frame, ax;

    record = &entity_table[entity_id];
    resolved = resource_handle_resolve(resource_handle);
    if (resolved.loaded == 0u) {
        goto restore_registers;
    }

    resource = (const volatile entity_resource far *)MK_FP(
            resolved.segment, resolved.offset);
    if ((i16)selected_frame >= resource->frame_count) {
        goto restore_registers;
    }

    record->flags = (u16)(
            (resource->flags & ENTITY_RESOURCE_FLAG)
            | ENTITY_ACTIVATE_FLAGS);

    packed_frame = resource->packed_frame_offsets[selected_frame];
    frame_segment = (u16)(
            resolved.segment + (u16)(packed_frame >> 4));
    frame_offset = (u16)(
            resolved.offset
            + (u16)sizeof(resource->flags)
            + (u16)sizeof(resource->frame_count)
            + (u16)(packed_frame & 0x0fu));
    frame = (const volatile sprite_frame far *)MK_FP(
            frame_segment, frame_offset);
    record->frame = frame;

    record->extent_width = frame->stride;
    if (record->committed_extent_width == 0u) {
        record->committed_extent_width = record->extent_width;
    }
    record->extent_height = frame->height;
    if (record->committed_extent_height == 0u) {
        record->committed_extent_height = record->extent_height;
    }
    record->draw_x = draw_x;
    record->draw_y = draw_y;

restore_registers:
    _asm pop ds;
    _asm pop es;
    _asm pop dx;
    _asm pop ax;
}
