#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_slot1_state_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    if (state->field_040 > 0x0080u ||
            state->field_038 > 0x0040 || state->field_038 < -0x0040 ||
            state->field_03c > 0x0040 || state->field_03c < -0x0040) {
        xdb_scrut_slot1_motion_continuation(state, context);
        return;
    }

    xdb_alien_control_latch = (xdb_u16)(size_t)context;
    if ((xdb_scrut_slot1_selection_state & 3u) != 0u) {
        xdb_scrut_slot1_camera_update(state, context);
        return;
    }

    xdb_scrut_slot1_selection_state = 1;
    state->owner_offset = 0x25a8u;
    state->field_054 = 0;
    state->position_x = (xdb_i32)((xdb_u32)state->position_x & 0xffff0000UL);
    state->position_y = (xdb_i32)((xdb_u32)state->position_y & 0xffff0000UL);
    state->position_z = (xdb_i32)(
            ((xdb_u32)state->position_z & 0xffff0000UL) | 0x20UL);
    xdb_alien_palette_pulse_1.value = (xdb_i32)(
            (xdb_u32)xdb_alien_palette_pulse_1.value + 0x1eUL);
    xdb_alien_palette_pulse_2.value = (xdb_i32)(
            (xdb_u32)xdb_alien_palette_pulse_2.value + 0x23UL);
    state->callback = xdb_scrut_slot1_wave_update;
    xdb_alien_callback_countdown = 5;
}
