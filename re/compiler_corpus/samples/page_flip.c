/* Codegen probe for BLOODPRG 0x00954A. */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef volatile u8 FAR *buffer_ptr;

extern volatile u8 palette_dirty;
extern volatile u8 transparent_zero;
extern volatile u8 dirty_copy;
extern volatile u16 ship_flags;
extern volatile u16 bridge_frame;
extern buffer_ptr display_buffer;
extern buffer_ptr back_buffer;

void FAR blit_fill_row_probe(u8 color);
void FAR projection_matrix_build_probe(void);
void FAR point_cloud_project_probe(void);
void FAR object_sprite_project_probe(void);
void FAR sprite_slot_commit_probe(u16 first, u16 last);
void FAR sprite_slot_render_probe(u16 first, u16 last);
void NEAR bridge_frame_load_probe(u16 frame);

#if defined(__WATCOMC__)
#pragma aux blit_fill_row_probe parm [ax] modify exact []
#pragma aux projection_matrix_build_probe modify exact []
#pragma aux point_cloud_project_probe modify exact []
#pragma aux object_sprite_project_probe modify exact []
#pragma aux sprite_slot_commit_probe parm [ax] [bx] modify exact []
#pragma aux sprite_slot_render_probe parm [ax] [bx] modify exact []
#pragma aux bridge_frame_load_probe parm [ax] modify exact []
#pragma aux page_flip_probe value [ax] modify exact [ax bx]
#endif

u16 FAR page_flip_probe(void)
{
    buffer_ptr saved_display_buffer;
    volatile u16 first_object_id;
    volatile u16 last_object_id;
    u16 frame;

    palette_dirty = 1;
    saved_display_buffer = display_buffer;
    display_buffer = back_buffer;

    blit_fill_row_probe(0);
    projection_matrix_build_probe();
    point_cloud_project_probe();
    object_sprite_project_probe();
    first_object_id = 0x0015u;
    last_object_id = 0x001fu;
    sprite_slot_commit_probe(first_object_id, last_object_id);
    sprite_slot_render_probe(first_object_id, last_object_id);

    display_buffer = saved_display_buffer;
    if ((ship_flags & 1u) != 0) {
        return first_object_id;
    }

    transparent_zero = 1;
    dirty_copy = 1;
    frame = bridge_frame;
    bridge_frame_load_probe(frame);
    return frame;
}
