#include "../include/xdb_alien.h"
#include "../include/xdb_mouse.h"

void XDB_NEAR xdb_scrut_mouse_camera_step(void)
{
    xdb_i16 horizontal;
    xdb_i16 vertical;
    xdb_u16 depth_step;
    xdb_u16 key;

    xdb_mouse_driver_read_state(&xdb_alien_mouse_state);
    xdb_alien_mouse_x = (xdb_u16)(xdb_alien_mouse_x - 0x0140u);
    xdb_alien_mouse_y = (xdb_u16)(xdb_alien_mouse_y - 0x0200u);

    horizontal = (xdb_i16)xdb_alien_mouse_x;
    horizontal >>= 1;
    horizontal = (xdb_i16)((xdb_u16)horizontal - 5u);
    if (horizontal < 0) {
        horizontal = (xdb_i16)((xdb_u16)horizontal + 10u);
        if (horizontal >= 0) {
            horizontal = 0;
        }
    }
    horizontal = (xdb_i16)(
            (xdb_u16)horizontal - (xdb_u16)xdb_alien_mouse_filter_x);
    horizontal >>= 1;
    xdb_alien_mouse_filter_x = horizontal;
    xdb_alien_camera_pan = (xdb_i16)(
            (xdb_u16)xdb_alien_camera_pan + (xdb_u16)horizontal);
    horizontal = (xdb_i16)((xdb_u16)horizontal << 3);
    horizontal = (xdb_i16)(
            (xdb_u16)horizontal -
            (xdb_u16)xdb_alien_camera_pan_secondary);
    horizontal >>= 1;
    xdb_alien_camera_pan_secondary = (xdb_i16)(
            (xdb_u16)xdb_alien_camera_pan_secondary +
            (xdb_u16)horizontal);

    vertical = (xdb_i16)(0u - xdb_alien_mouse_y);
    vertical = (xdb_i16)((xdb_u16)vertical - 5u);
    if (vertical < 0) {
        vertical = (xdb_i16)((xdb_u16)vertical + 10u);
        if (vertical >= 0) {
            vertical = 0;
        }
    }
    vertical = (xdb_i16)((xdb_u16)vertical << 1);
    vertical = (xdb_i16)(
            (xdb_u16)vertical - (xdb_u16)xdb_alien_camera_pitch);
    vertical >>= 4;
    xdb_alien_camera_pitch = (xdb_i16)(
            (xdb_u16)xdb_alien_camera_pitch + (xdb_u16)vertical);

    depth_step = (xdb_u16)xdb_alien_camera_depth_step;
    if (xdb_alien_mouse_buttons & 1u) {
        depth_step = (xdb_u16)(depth_step + 10u);
    }
    if (xdb_alien_mouse_buttons & 2u) {
        depth_step = (xdb_u16)(
                depth_step - (xdb_u16)((xdb_i16)depth_step >> 3) - 1u);
    }
    if ((xdb_i16)depth_step <= -8) {
        depth_step = (xdb_u16)(depth_step + 8u);
        if (xdb_alien_control_latch != 0) {
            depth_step = (xdb_u16)(depth_step - 0x40u);
        }
    } else if (xdb_alien_control_latch != 0) {
        depth_step = (xdb_u16)-100;
    }
    xdb_alien_camera_depth_step = (xdb_i16)depth_step;

    key = xdb_alien_key_event;
    xdb_alien_key_event = 0;
    if (key == 0x4800u) {
        xdb_alien_camera_depth_step = (xdb_i16)(
                (xdb_u16)xdb_alien_camera_depth_step + 8u);
    } else if (key == 0x5000u) {
        xdb_alien_camera_depth_step = (xdb_i16)(
                (xdb_u16)xdb_alien_camera_depth_step - 8u);
    } else if ((xdb_u8)key == 0x20u) {
        xdb_alien_code_flags |= 0x10u;
    }
}
