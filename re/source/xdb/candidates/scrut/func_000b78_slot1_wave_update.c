#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_slot1_wave_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 sample;

    if ((xdb_scrut_slot2_active & 1u) == 0u) {
        state->field_04e = 0;
        state->field_050 = 0x0800u;
        state->field_052 = (xdb_i16)((xdb_u16)state->field_052 + 0x0035u);
        if ((xdb_scrut_slot1_selection_state & 2u) == 0u) {
            return;
        }

        sample = (xdb_u16)((xdb_u16)xdb_alien_method_delta + 8u);
        if (sample >= 0x0080u) {
            sample = 0x007fu;
        }
        xdb_alien_method_delta = (xdb_i16)sample;
        state->owner_offset = (xdb_u16)(size_t)xdb_scrut_slot1_selected_state;
        state->callback = xdb_scrut_slot1_finish_update;
        xdb_scrut_slot1_selection_state = 0;
        xdb_alien_palette_pulse_1.value = (xdb_i32)(
                (xdb_u32)xdb_alien_palette_pulse_1.value - 0x1eUL);
        xdb_alien_palette_pulse_2.value = (xdb_i32)(
                (xdb_u32)xdb_alien_palette_pulse_2.value - 0x23UL);
        xdb_scrut_slot2_active = 0;
        xdb_alien_callback_countdown = 4;
        return;
    }

    xdb_scrut_slot1_selection_state = 0;
    xdb_alien_palette_pulse_1.value = (xdb_i32)(
            (xdb_u32)xdb_alien_palette_pulse_1.value - 0x1eUL);
    xdb_alien_palette_pulse_2.value = (xdb_i32)(
            (xdb_u32)xdb_alien_palette_pulse_2.value - 0x23UL);
    state->owner_offset = 0x22a8u;
    state->position_x = -(xdb_i32)xdb_alien_view_x;
    state->position_y = -(xdb_i32)xdb_alien_view_y;
    state->position_z = -(xdb_i32)xdb_alien_view_z;
    xdb_scrut_slot1_camera_update(state, context);
}
