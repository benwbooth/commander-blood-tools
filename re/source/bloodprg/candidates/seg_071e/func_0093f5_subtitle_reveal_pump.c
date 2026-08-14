#include <dos.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_vm.h"

#define SUBTITLE_HOLD_OWNER 0x5e64u
#define SUBTITLE_OPENING_PHASE 2u
#define SUBTITLE_LINE_PITCH 8u

void CB_FAR subtitle_reveal_pump(void)
{
    const bloodprg_subtitle_frame_primitive CB_NEAR *primitive;
    const volatile cb_u8 CB_NEAR *character;
    const cb_u8 CB_NEAR *line;
    cb_u16 phase;
    cb_u16 y;
    cb_u8 color;

    if ((vm_subtitle_display_mode_ds & 2u) == 0u
            && (vm_text_display_active & 1u) == 0u
            && ((vm_presentation_hold_ready & 1u) == 0u
                || vm_presentation_owner_offset != SUBTITLE_HOLD_OWNER)) {
        return;
    }

    line = (const cb_u8 CB_NEAR *)vm_text_buffer;
    if (subtitle_reveal_cursor == 0u) {
        subtitle_reveal_delay = 2u;
        subtitle_opening_frame_pulse = 1u;
        subtitle_reveal_cursor = FP_OFF(line);
        vm_text_reveal_phase = SUBTITLE_OPENING_PHASE;
    }

    phase = vm_text_reveal_phase;
    color = 0xffu;
    primitive = subtitle_frame_primitives_primary;
    if (phase != SUBTITLE_OPENING_PHASE) {
        color = 0xfeu;
        if (phase != 1u) {
            primitive = subtitle_frame_primitives_secondary;
            graphics_span_remap_enabled = 1u;
        }
    }

    while (primitive->kind >= 0) {
        if (primitive->kind == 1) {
            gfx_clipped_planar_vertical_span(
                    color,
                    primitive->x,
                    primitive->y,
                    primitive->extent);
        } else {
            gfx_clipped_span_fill(
                    color,
                    primitive->x,
                    primitive->y,
                    primitive->extent);
        }
        ++primitive;
    }
    graphics_span_remap_enabled = 0u;

    if (phase != 0u) {
        if (subtitle_opening_frame_pulse == 0u) {
            subtitle_opening_frame_pulse = 1u;
            vm_text_reveal_phase = (cb_u16)(phase - 1u);
        }
        return;
    }

    character = (const volatile cb_u8 CB_NEAR *)subtitle_reveal_cursor;
    if (*character != 0u) {
        if (subtitle_reveal_delay == 0u) {
            subtitle_reveal_delay = subtitle_text_speed_step >> 2;
            ++subtitle_reveal_cursor;
        }
    } else if ((vm_ship_active_flags_low & 4u) == 0u
            && (vm_dialogue_hold_complete & 1u) == 0u
            && (vm_presentation_hold_ready & 1u) == 0u) {
        vm_text_voice_trigger = 0u;
        vm_dialogue_hold_countdown =
                (cb_u16)(subtitle_text_speed_step << 2);
        vm_dialogue_hold_complete = 1u;
    }

    y = presentation_text_origin_y;
    for (;;) {
        subtitle_reveal_draw_wrapper(
                line, presentation_text_origin_x, y);
        while (*line != '\r') {
            ++line;
        }
        ++line;
        if (*line == 0u) {
            return;
        }
        y = (cb_u16)(y + SUBTITLE_LINE_PITCH);
    }
}
