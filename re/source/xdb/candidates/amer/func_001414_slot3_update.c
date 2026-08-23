#include "../include/xdb_alien.h"

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

static xdb_i16 sar16_with_carry(
        xdb_i16 value, unsigned shift, xdb_u16 *carry)
{
    xdb_u16 bits = (xdb_u16)value;

    *carry = 0;
    while (shift-- != 0u) {
        *carry = (xdb_u16)(bits & 1u);
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

static void slot3_feedback_sample(
        xdb_alien_biased_state XDB_NEAR *state)
{
    xdb_u16 cursor = (xdb_u16)(state->field_058 + 0x0028u);
    xdb_i16 sample;

    cursor &= 0x0ffcu;
    state->field_058 = cursor;
    sample = *(volatile xdb_i16 XDB_NEAR *)(xdb_alien_motion_samples + cursor);
    (void)sar16(sample, 5u);
}

void XDB_NEAR xdb_amer_slot3_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 ring_cursor;
    xdb_u16 ring_index;

    (void)context;
    ring_cursor = state->ring_offset;
    ring_index = (xdb_u16)(ring_cursor >> 3);
    state->field_04e = (xdb_i16)(
            (xdb_u16)state->field_04e
            + (xdb_u16)xdb_amer_slot3_ring[ring_index].field_000);
    state->field_050 = (xdb_u16)(
            state->field_050
            + (xdb_u16)xdb_amer_slot3_ring[ring_index].field_002);
    state->field_054 = xdb_amer_slot3_ring[ring_index].field_004;
    if (xdb_amer_slot3_timer != 0u) {
        return;
    }

    ring_cursor = (xdb_u16)((state->ring_offset + 8u) & 0x03fcu);
    state->ring_offset = ring_cursor;
    ring_index = (xdb_u16)(ring_cursor >> 3);
    if ((xdb_amer_slot3_ring[ring_index].field_006 & 3) != 0) {
        if ((xdb_amer_slot3_ring[ring_index].field_006 & 2) != 0) {
            xdb_amer_slot3_capture_resume_state(state, context);
            return;
        }

        {
            xdb_u16 queue_cursor = xdb_amer_slot11_queue_cursor;

            xdb_amer_slot11_state_queue[queue_cursor >> 1] =
                    (xdb_u16)(size_t)state;
            queue_cursor = (xdb_u16)((queue_cursor + 2u) & 0x000fu);
            (void)queue_cursor;
            xdb_amer_slot11_current_state = (xdb_u16)(size_t)state;
        }
        if (state->field_05c == 0u) {
            xdb_u16 object_offset =
                    *(volatile xdb_u16 XDB_NEAR *)
                    ((volatile xdb_u8 XDB_NEAR *)state + 6u);
            xdb_u16 object_count =
                    *(volatile xdb_u16 XDB_NEAR *)
                    ((volatile xdb_u8 XDB_NEAR *)state + 2u);
            volatile xdb_u32 XDB_FAR *object = XDB_FAR_AT(
                    xdb_u32, xdb_alien_object_segment, object_offset);

            while (object_count-- != 0u) {
                *object -= 0x00800080UL;
                object += 5;
            }
        }
        xdb_amer_slot3_restart_initial_update(state, context);
        return;
    }

    if (state->field_05c != 0u
            || state->field_040 > 0x0040u
            || state->field_038 > 0x0040
            || state->field_038 < (xdb_i16)0xffc0u
            || state->field_03c > 0x0040
            || state->field_03c < (xdb_i16)0xffc0u) {
        slot3_feedback_sample(state);
        return;
    }

    xdb_alien_control_latch = 1;
    if (xdb_alien_callback_countdown == 0u) {
        xdb_alien_callback_countdown = 2;
    }
    xdb_amer_slot3_ring[ring_index].field_004 = 8;
    if ((xdb_amer_slot1_selection_state & 3u) != 0u) {
        slot3_feedback_sample(state);
        return;
    }

    xdb_amer_slot3_ring[ring_index].field_006 = 1;
    xdb_amer_slot1_state_update(state, context);
}
