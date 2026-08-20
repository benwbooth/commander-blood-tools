#include "../include/xdb_alien.h"

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
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

void XDB_NEAR xdb_scrut_slot3_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_ring_entry XDB_CODE_DATA *ring;
    xdb_u16 ring_cursor;

    ring_cursor = state->ring_offset;
    ring = &xdb_scrut_slot3_ring[ring_cursor >> 3];
    state->field_04e = (xdb_i16)(
            (xdb_u16)state->field_04e + (xdb_u16)ring->field_000);
    state->field_050 = (xdb_u16)(
            state->field_050 + (xdb_u16)ring->field_002);
    state->field_054 = ring->field_004;
    if (xdb_scrut_slot3_timer != 0u) {
        return;
    }

    ring_cursor = (xdb_u16)((state->ring_offset + 8u) & 0x03fcu);
    state->ring_offset = ring_cursor;
    ring = &xdb_scrut_slot3_ring[ring_cursor >> 3];
    if ((ring->field_006 & 3) != 0) {
        if ((ring->field_006 & 2) != 0) {
            xdb_scrut_slot3_capture_resume_state(state, context);
            return;
        }

        {
            xdb_u16 queue_cursor = xdb_scrut_slot11_queue_cursor;

            xdb_scrut_slot11_state_queue[queue_cursor >> 1] =
                    (xdb_u16)(size_t)state;
            queue_cursor = (xdb_u16)((queue_cursor + 2u) & 0x000fu);
            (void)queue_cursor;
            xdb_scrut_slot11_current_state = (xdb_u16)(size_t)state;
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
        xdb_scrut_slot3_restart_initial_update(state, context);
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

    xdb_alien_control_latch = (xdb_u16)(size_t)context;
    if (xdb_alien_callback_countdown == 0u) {
        xdb_alien_callback_countdown = 2;
    }
    ring->field_004 = 8;
    if ((xdb_scrut_slot1_selection_state & 3u) != 0u) {
        slot3_feedback_sample(state);
        return;
    }

    ring->field_006 = 1;
    xdb_scrut_slot1_selection_state = 1;
    state->owner_offset = 0x25a8u;
    state->field_054 = 0;
    state->position_x = 0;
    state->position_y = 0;
    state->position_z = 0x20;
    xdb_alien_palette_pulse_1.value += 0x1e;
    xdb_alien_palette_pulse_2.value += 0x23;
    state->callback = xdb_scrut_slot1_wave_update;
    xdb_alien_callback_countdown = 5;
}
