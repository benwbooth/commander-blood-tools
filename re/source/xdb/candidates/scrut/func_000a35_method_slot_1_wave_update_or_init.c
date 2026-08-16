#include "../include/xdb_alien.h"

void XDB_NEAR xdb_scrut_method_slot_1_wave_update_or_init(
        xdb_alien_method_context XDB_NEAR *context)
{
    volatile xdb_alien_biased_state XDB_NEAR *state =
            (volatile xdb_alien_biased_state XDB_NEAR *)
            ((volatile xdb_u8 XDB_NEAR *)context->state +
             XDB_ALIEN_CURSOR_BIAS);
    xdb_alien_wave_object XDB_FAR *object;
    xdb_u16 count;
    xdb_u16 phase;
    xdb_u16 sample_offset;
    xdb_u16 scale;
    xdb_i16 sample;
    xdb_i16 step;
    xdb_i16 distance;
    xdb_i32 product;

    if (context->control.state == 0) {
        context->control.state = 1;
        xdb_scrut_slot1_selection_state = 0;
        context->continuation.slot1_wave.primary_phase = 4;
        context->continuation.slot1_wave.primary_step = 0x30;
        context->continuation.slot1_wave.secondary_phase = 4;
        context->continuation.slot1_wave.secondary_step = 0x10;
        state->field_054 = 0x0c;
        state->field_04e = 0;
        state->field_050 = 0;
        state->field_052 = 0;
        xdb_scrut_slot1_selected_state = state;
        return;
    }

    ++state->field_050;
    phase = context->continuation.slot1_wave.primary_phase & 0x0ffcu;
    sample = *(volatile xdb_i16 XDB_NEAR *)
            (xdb_alien_motion_samples + phase);
    sample >>= 8;
    xdb_scrut_slot1_current_sample = sample;

    if ((xdb_scrut_slot1_selection_state & 1u) != 0u) {
        xdb_i16 value = (xdb_i16)(
                (xdb_u16)sample - 60u +
                (xdb_u16)(xdb_i16)state->position_y +
                (xdb_u16)xdb_alien_view_y);
        if (value >= 0 && value <= 128) {
            value = (xdb_i16)(
                    (xdb_u16)(xdb_i16)state->position_x +
                    (xdb_u16)xdb_alien_view_x);
            if (value >= -256 && value <= 256) {
                value = (xdb_i16)(
                        (xdb_u16)(xdb_i16)state->position_z +
                        (xdb_u16)xdb_alien_view_z);
                if (value >= -256 && value <= 256) {
                    xdb_scrut_slot1_selection_state = 2;
                    xdb_scrut_slot1_selected_state = state;
                    context->continuation.slot1_wave.primary_step = 0x170;
                }
            }
        }
    }

    step = context->continuation.slot1_wave.primary_step;
    if (step > 0x30) {
        step = (xdb_i16)((xdb_u16)step - 4u);
        context->continuation.slot1_wave.primary_step = step;
    }
    phase = context->continuation.slot1_wave.primary_phase;
    context->continuation.slot1_wave.primary_phase =
            (xdb_u16)(phase + (xdb_u16)step);
    object = XDB_FAR_AT(
            xdb_alien_wave_object,
            xdb_alien_object_segment,
            context->object_offset);
    count = context->object_count;
    do {
        sample_offset =
                ((xdb_u16)(object->phase * 2u) + phase) & 0x0ffcu;
        sample = *(volatile xdb_i16 XDB_NEAR *)
                (xdb_alien_motion_samples + sample_offset);
        object->motion = (xdb_i16)(
                (xdb_u16)object->motion - (xdb_u16)(sample >> 8));
        sample_offset = (sample_offset + (xdb_u16)step) & 0x0ffcu;
        sample = *(volatile xdb_i16 XDB_NEAR *)
                (xdb_alien_motion_samples + sample_offset);
        object->motion = (xdb_i16)(
                (xdb_u16)object->motion + (xdb_u16)(sample >> 8));
        ++object;
    } while (--count != 0u);

    step = (xdb_i16)context->continuation.slot1_wave.secondary_step;
    phase = (xdb_u16)(
            context->continuation.slot1_wave.secondary_phase +
            (xdb_u16)step);
    context->continuation.slot1_wave.secondary_phase = phase;
    object = XDB_FAR_AT(
            xdb_alien_wave_object,
            xdb_alien_object_segment,
            context->object_offset);
    count = context->object_count;
    do {
        distance = (xdb_i16)((xdb_u16)object->distance - 25u);
        if (distance < 0) {
            distance = (xdb_i16)(0u - (xdb_u16)distance);
            distance = (xdb_i16)((xdb_u16)distance - 50u);
        }
        if (distance >= 0) {
            scale = (xdb_u16)distance * 2u;
            sample_offset = (scale + phase) & 0x0ffcu;
            sample = *(volatile xdb_i16 XDB_NEAR *)
                    (xdb_alien_motion_samples + sample_offset);
            product = (xdb_i32)sample * (xdb_u16)scale;
            object->motion = (xdb_i16)(
                    (xdb_u16)object->motion -
                    (xdb_u16)(xdb_i16)(product >> 17));
            sample_offset = (sample_offset + (xdb_u16)step) & 0x0ffcu;
            sample = *(volatile xdb_i16 XDB_NEAR *)
                    (xdb_alien_motion_samples + sample_offset);
            product = (xdb_i32)sample * (xdb_u16)scale;
            object->motion = (xdb_i16)(
                    (xdb_u16)object->motion +
                    (xdb_u16)(xdb_i16)(product >> 17));
        }
        ++object;
    } while (--count != 0u);
}

/* The original entry is shared by the method table and state callbacks. */
void XDB_NEAR xdb_scrut_slot1_wave_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    (void)state;
    xdb_scrut_method_slot_1_wave_update_or_init(context);
}
