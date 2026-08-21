#include <conio.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_input.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#if defined(__WATCOMC__)
#pragma intrinsic(outp)
#endif

#define NAV_CHOICE_GATE_FLAG 0x01u
#define NAV_CHOICE_UI_BUSY_FLAG 0x08u
#define NAV_CHOICE_UI_ACTIVATE_FLAGS 0x0Cu
#define NAV_CHOICE_FRAME_MIN 40
#define NAV_CHOICE_FRAME_MAX 60
#define NAV_CHOICE_FRAME_CENTER 45
#define NAV_CHOICE_RIGHT_BASE 287u
#define NAV_CHOICE_WIDTH 110u
#define NAV_CHOICE_Y_BASE 72u
#define NAV_CHOICE_ROW_HEIGHT 18u
#define NAV_CHOICE_ROW_COUNT 5u
#define NAV_CHOICE_BASE_PALETTE_INDEX 123u
#define NAV_CHOICE_ACTIVE_RED 63u
#define NAV_CHOICE_TARGET_Y_BASE 80u
#define NAV_CHOICE_HOLD_TICKS 90u
#define NAV_CHOICE_LAYOUT_CENTER_X 100u
#define NAV_CHOICE_INTERPOLATION_TICKS 10u
#define NAV_CHOICE_SOUND 4

void CB_NEAR nav_choice_dispatch(void)
{
    cb_i16 frame;
    cb_i16 relative_frame;
    cb_i16 right;
    cb_i16 left;
    cb_i16 y_offset;
    cb_u16 distance;
    cb_u16 quarter_distance;
    cb_u16 y_origin;
    cb_u16 selection;
    cb_u16 target_y;
    cb_u8 row_height;
    cb_u8 row;
    cb_u8 palette_row;
    cb_u8 blocked;

    if ((vm_c2_presentation_gate & NAV_CHOICE_GATE_FLAG) != 0u) {
        return;
    }
    blocked = nav_choice_left_motion_active;
    blocked |= nav_choice_right_motion_active;
    blocked |= presentation_choice_active;
    blocked |= ship_3d_nav_choice_sound_gate;
    if (blocked != 0u) {
        return;
    }
    if ((vm_presentation_active & NAV_CHOICE_GATE_FLAG) != 0u) {
        return;
    }

    selection = nav_console_selected_item;
    if (selection == 0u) {
        frame = vm_bridge_view_frame;
        if (frame > NAV_CHOICE_FRAME_MAX || frame < NAV_CHOICE_FRAME_MIN) {
            return;
        }

        video_retrace_phase_wait();
        outp(0x03C8u, NAV_CHOICE_BASE_PALETTE_INDEX);
        for (palette_row = 0u; palette_row < NAV_CHOICE_ROW_COUNT; ++palette_row) {
            outp(0x03C9u, 16u);
            outp(0x03C9u, 12u);
            outp(0x03C9u, 0u);
        }

        relative_frame = (cb_i16)(frame - NAV_CHOICE_FRAME_CENTER);
        right = (cb_i16)(NAV_CHOICE_RIGHT_BASE -
                ((cb_u16)relative_frame << 3));
        if (mouse_x > right) {
            return;
        }
        left = (cb_i16)((cb_u16)right - NAV_CHOICE_WIDTH);
        if (left < 0 || mouse_x < left) {
            return;
        }

        if (relative_frame < 0) {
            distance = (cb_u16)-relative_frame;
        } else {
            distance = (cb_u16)relative_frame;
        }
        quarter_distance = distance >> 2;
        y_origin = NAV_CHOICE_Y_BASE + distance + quarter_distance;
        row_height = (cb_u8)(NAV_CHOICE_ROW_HEIGHT -
                ((cb_u8)quarter_distance >> 1));
        y_offset = (cb_i16)((cb_u16)mouse_y - y_origin);
        if (y_offset < 0) {
            return;
        }
        row = (cb_u8)((cb_u16)y_offset / row_height);
        if ((cb_i8)row >= (cb_i8)NAV_CHOICE_ROW_COUNT) {
            return;
        }

        outp(0x03C8u, row + NAV_CHOICE_BASE_PALETTE_INDEX);
        outp(0x03C9u, NAV_CHOICE_ACTIVE_RED);
        outp(0x03C9u, 0u);
        outp(0x03C9u, 0u);
        if ((mouse_primary_pressed & BLOODPRG_MOUSE_BUTTON_PRIMARY) == 0u) {
            return;
        }

        nav_actor_presentation_state = 5u;
        selection = row + 1u;
        nav_console_selected_item = selection;
        vm_ui_flags |= NAV_CHOICE_UI_ACTIVATE_FLAGS;
        nav_bridge_seek_target_arc = NAV_CHOICE_HOLD_TICKS;
        nav_choice_phase = 1u;
        target_y = NAV_CHOICE_TARGET_Y_BASE +
                (cb_u16)row * NAV_CHOICE_ROW_HEIGHT;
        ship_3d_nav_choice_target_y = target_y;
        ship_3d_target_layout_preserve_widths = 1u;
        ship_3d_target_layout_center_x = NAV_CHOICE_LAYOUT_CENTER_X;
        ship_3d_target_layout_extra_entry = 1u;
        ship_3d_interpolation_duration = NAV_CHOICE_INTERPOLATION_TICKS;
        snd_play_clip(NAV_CHOICE_SOUND);
    }

    if ((vm_ui_flags & NAV_CHOICE_UI_BUSY_FLAG) == 0u) {
        switch (selection) {
        case 1u:
            nav_choice_handler_0();
            break;
        case 2u:
            nav_choice_handler_1();
            break;
        case 3u:
            nav_choice_handler_2();
            break;
        case 4u:
            nav_choice_handler_3();
            break;
        case 5u:
            nav_choice_handler_4();
            break;
        default:
            break;
        }
    }
}
