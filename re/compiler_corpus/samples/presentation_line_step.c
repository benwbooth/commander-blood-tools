/*
 * Codegen probe for BLOODPRG 0x007E1C.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#define LINE_LOADED_FLAG 0x04u
#define UI_BUSY_GATE 0x08u
#define UI_REDRAW_FLAG 0x04u

typedef struct resource_header {
    u16 field_00;
    u16 terminal_frame;
} resource_header;

typedef struct line_record {
    u8 flags;
    u8 pad_01;
    u16 resource_id;
    u16 pad_04;
    u16 terminal_frame;
    u16 frame_index;
    u8 pad_0a[10];
    u16 draw_x;
    u16 draw_y;
} line_record;

extern volatile u8 ui_flags;
extern volatile u8 reverse_playback;
extern volatile u8 FAR *resource_buffer;
extern const volatile char FAR resource_names[][16];

unsigned long FAR resource_file_load_probe(const volatile char FAR *path,
        volatile u8 FAR *destination);
void FAR entity_record_setter_probe(u16 entity_id,
        const volatile void FAR *resource,
        u16 draw_x,
        u16 draw_y,
        u16 frame_index);

int NEAR presentation_line_step_probe(volatile line_record *line)
{
    const volatile resource_header FAR *resource;
    u16 frame;

    if ((ui_flags & UI_BUSY_GATE) != 0) {
        return 0;
    }

    if ((line->flags & LINE_LOADED_FLAG) == 0) {
        ui_flags = (u8)(ui_flags | UI_REDRAW_FLAG);
        resource_file_load_probe(resource_names[line->resource_id], resource_buffer);
        resource = (const volatile resource_header FAR *)resource_buffer;
        line->terminal_frame = resource->terminal_frame;
        frame = (u16)(line->terminal_frame - 1u);
        if ((reverse_playback & 1u) == 0) {
            frame = 0;
            reverse_playback = 0;
        }
        line->frame_index = frame;
        line->flags = (u8)(line->flags | LINE_LOADED_FLAG);
    }

    entity_record_setter_probe(4u, resource_buffer,
        line->draw_x, line->draw_y, line->frame_index);

    if ((reverse_playback & 1u) != 0) {
        frame = line->frame_index;
        if (frame == 0) {
            reverse_playback = 0;
            ui_flags = (u8)(ui_flags & 0xfbu);
            return 1;
        }
        line->frame_index = (u16)(frame - 1u);
    } else {
        frame = line->frame_index;
        if (frame == line->terminal_frame) {
            reverse_playback = 0;
            ui_flags = (u8)(ui_flags & 0xfbu);
            return 1;
        }
        line->frame_index = (u16)(frame + 1u);
    }

    return 0;
}
