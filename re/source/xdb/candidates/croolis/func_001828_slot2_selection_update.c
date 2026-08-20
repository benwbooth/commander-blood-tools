#include "../include/xdb_alien.h"

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

static volatile xdb_i16 XDB_NEAR *state_word(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_u16 offset)
{
    return (volatile xdb_i16 XDB_NEAR *)
            ((volatile xdb_u8 XDB_NEAR *)state + offset);
}

static xdb_i16 clamp_height(xdb_i16 value)
{
    if (value > 0x0300) {
        return 0x0300;
    }
    if (value < (xdb_i16)0xfd00u) {
        return (xdb_i16)0xfd00u;
    }
    return value;
}

void XDB_NEAR xdb_croolis_slot2_selection_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_state_cursor selected;
    xdb_i32 score;
    xdb_i16 value;
    xdb_i16 phase;
    int clear_selected = 0;

    if ((xdb_croolis_slot1_selection_state & 3u) == 0u) {
        state->callback = xdb_croolis_slot2_update;
        context->continuation.croolis_slot2.duration = 0;
        xdb_croolis_slot2_active = 0;
        clear_selected = 1;
        goto reset_motion;
    }

    selected = xdb_croolis_slot2_selected_state;
    if (selected != NULL && selected != state) {
        goto reset_motion;
    }
    if (state->field_040 > 0x05dcu
            || (xdb_i16)(xdb_u16)state->field_032 > (xdb_i16)0xb000u
            || state->field_038 > 0x01f4
            || state->field_038 < (xdb_i16)0xfe0cu) {
        clear_selected = 1;
        goto reset_motion;
    }
    if (xdb_alien_control_latch == (xdb_u16)(size_t)context) {
        xdb_croolis_slot2_active = 1;
        clear_selected = 1;
        goto reset_motion;
    }

    xdb_croolis_slot2_selected_state = state;
    score = (xdb_i32)state->field_038 * state->field_032
            - ((xdb_i32)(xdb_i16)state->field_040 + 0x64L)
                    * state->field_01a;
    value = score < 0 ? 0x30 : (xdb_i16)0xffd0u;
    value = (xdb_i16)((xdb_u16)state->field_052 + (xdb_u16)value);
    state->field_052 = clamp_height(value);
    state->field_050 = (xdb_u16)(state->field_050
            + (xdb_u16)sar16(state->field_052, 4u));

    value = (xdb_i16)((xdb_u16)*state_word(state, 0x46)
            + (xdb_u16)xdb_alien_view_y);
    value = sar16(value, 1u);
    value = (xdb_i16)((xdb_u16)value - (xdb_u16)state->field_04e);
    value = (xdb_i16)(state->field_04e + sar16(value, 2u));
    state->field_04e = clamp_height(value);

    value = (xdb_i16)(0x00c8u - (xdb_u16)state->field_054);
    state->field_054 = (xdb_i16)(state->field_054 + sar16(value, 2u));
    phase = (xdb_i16)(state->field_056 - 0x10);
    if (phase < 0) {
        xdb_alien_callback_countdown = 1;
    }
    phase = (xdb_i16)((xdb_u16)phase & 0x007fu);
    state->field_056 = phase;
    *state_word(state, 0x0220) = (xdb_i16)(
            (xdb_u16)*state_word(state, 0x022c) + (xdb_u16)phase);
    *state_word(state, 0x027e) = (xdb_i16)(
            (xdb_u16)*state_word(state, 0x028a) + (xdb_u16)phase);
    *state_word(state, 0x02dc) = (xdb_i16)(
            (xdb_u16)*state_word(state, 0x02e8) + (xdb_u16)phase);
    return;

reset_motion:
    if (clear_selected) {
        xdb_croolis_slot2_selected_state = NULL;
    }
    *state_word(state, 0x0220) = *state_word(state, 0x022c);
    *state_word(state, 0x027e) = *state_word(state, 0x028a);
    *state_word(state, 0x02dc) = *state_word(state, 0x02e8);
    xdb_croolis_slot2_reset_or_camera(state, context, 0x03e8L);
}
