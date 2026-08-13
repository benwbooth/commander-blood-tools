#include <string.h>

#include "../include/xdb_alien.h"
#include "../include/xdb_keyboard.h"
#include "../include/xdb_mouse.h"
#include "../include/xdb_video.h"

#define XDB_ALIEN_PALETTE_SIZE 0x0300u
#define XDB_ALIEN_FRAMEBUFFER_SIZE 0x3e80u

void XDB_FAR xdb_amer_main(void)
{
    xdb_u16 saved_data_segment = xdb_alien_data_segments_install(
            xdb_amer_data_segment);
    xdb_u16 running = 1u;

    xdb_alien_exit_requested = 0u;
    xdb_amer_vga_clear_and_sync();
    xdb_port_write_u8(0x03c8u, 0u);
    xdb_port_write_buffer_u8(
            0x03c9u,
            xdb_alien_display_palette,
            XDB_ALIEN_PALETTE_SIZE);
    xdb_amer_mouse_bounds_set(640u, 1024u);
    xdb_amer_mouse_position_set(320u, 512u);

    xdb_alien_frame_state = 0u;
    xdb_alien_view_x = 0x075d;
    xdb_alien_view_y = (xdb_i16)0xff11u;
    xdb_alien_view_z = (xdb_i16)0xd9c2u;
    xdb_alien_camera_pitch = 0;
    xdb_alien_camera_pan = 0x0678;
    xdb_alien_camera_pan_secondary = 0;
    xdb_alien_camera_depth_step = 0;
    xdb_alien_last_callback_clock = xdb_alien_frame_clock - 620ul;

    while (running != 0u) {
        volatile xdb_u16 XDB_FAR *framebuffer = XDB_FAR_AT(
                volatile xdb_u16,
                xdb_alien_framebuffer_segment,
                0u);
        volatile xdb_u16 XDB_NEAR *context_offset;
        xdb_video_page page;
        xdb_u16 display_page;
        xdb_i16 event;
        xdb_u32 clock;

        xdb_direction_forward();
        xdb_port_write_u16(0x03c4u, 0x0f02u);
        _fmemset(
                (void XDB_FAR *)framebuffer,
                0,
                XDB_ALIEN_FRAMEBUFFER_SIZE);
        xdb_amer_mouse_camera_step();
        xdb_amer_camera_matrix_update();
        xdb_amer_project_primary_mesh_then_render();
        xdb_amer_render_starfield();

        context_offset = xdb_alien_render_context_offsets;
        do {
            xdb_alien_method_context XDB_NEAR *context =
                    (xdb_alien_method_context XDB_NEAR *)*context_offset;

            ++context_offset;
            xdb_alien_active_projection_context =
                    (xdb_alien_projection_context XDB_NEAR *)context;
            (*(xdb_alien_method_callback XDB_NEAR *)(
                    xdb_alien_method_table
                    + context->method_table_offset))(context);
            xdb_amer_transform_and_project();
        } while (*context_offset != 0u);

        xdb_amer_bucket_faces_then_render();

        page.word = xdb_video_page_4000;
        display_page = page.word;
        page.byte.high = (xdb_u8)(page.byte.high + 0x40u);
        xdb_video_page_4000 = page.word;
        xdb_port_write_u16(
                0x03d4u,
                (xdb_u16)((display_page & 0xff00u) | 0x000cu));
        page.byte.high = (xdb_u8)((page.byte.high >> 4) | 0xa0u);
        xdb_alien_framebuffer_segment = page.word;

        if (xdb_alien_exit_requested != 0u) {
            break;
        }

        xdb_alien_frame_clock += 8ul;
        event = (xdb_i16)(xdb_alien_callback_countdown - 1u);
        xdb_alien_callback_countdown = 0u;
        clock = xdb_alien_frame_clock;
        if (event >= 0) {
            xdb_alien_frame_callback_invoke((xdb_u16)event, clock);
            xdb_alien_last_callback_clock = clock;
        } else if (clock - xdb_alien_last_callback_clock >= 600ul) {
            xdb_u32 callback_clock = clock - 1000ul;

            if (xdb_alien_control_latch != 0u) {
                xdb_alien_frame_callback_invoke(2u, clock);
                callback_clock = clock;
            }
            xdb_alien_last_callback_clock = callback_clock;
        }

        while (xdb_keyboard_ready() != 0u) {
            xdb_u16 key = xdb_keyboard_read();
            xdb_u8 character = (xdb_u8)key;

            xdb_alien_key_event = key;
            if (character == 'p' || character == 'P') {
                do {
                    key = xdb_keyboard_read();
                    character = (xdb_u8)key;
                } while (character != 'p' && character != 'P');
                break;
            }
            if (character == 0x1bu) {
                running = 0u;
                break;
            }
        }
    }

    xdb_amer_vga_clear_and_sync();
    xdb_amer_mouse_bounds_set(3000u, 200u);
    xdb_alien_data_segment_restore(saved_data_segment);
}
