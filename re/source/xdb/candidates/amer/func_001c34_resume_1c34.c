#include "../include/xdb_alien.h"

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

static void resume_apply_object_delta(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 delta = (xdb_i16)context->continuation.resume_state.value;
    volatile xdb_i16 XDB_FAR *object;

    object = XDB_FAR_AT(xdb_i16, xdb_alien_object_segment,
            (xdb_u16)(context->object_offset + 0x0002u));
    *object = (xdb_i16)((xdb_u16)*object + (xdb_u16)delta);
    object = XDB_FAR_AT(xdb_i16, xdb_alien_object_segment,
            (xdb_u16)(context->object_offset + 0x0426u));
    *object = (xdb_i16)((xdb_u16)*object - (xdb_u16)delta);
    object = XDB_FAR_AT(xdb_i16, xdb_alien_object_segment,
            (xdb_u16)(context->object_offset + 0x02beu));
    *object = (xdb_i16)((xdb_u16)*object + (xdb_u16)delta);
    object = XDB_FAR_AT(xdb_i16, xdb_alien_object_segment,
            (xdb_u16)(context->object_offset + 0x01f6u));
    *object = (xdb_i16)((xdb_u16)*object - (xdb_u16)delta);

    {
        xdb_u8 low = (xdb_u8)context->continuation.resume_state.value;
        xdb_u8 high = (xdb_u8)(
                context->continuation.resume_state.value >> 8);
        xdb_u8 adjusted_low = low;

        high = (xdb_u8)(high + low);
        if (high == 0) {
            adjusted_low = 2;
        }
        if ((xdb_i8)high >= 0x16) {
            adjusted_low = 0xfe;
        }
        context->continuation.resume_state.value =
                (xdb_u16)(((xdb_u16)high << 8) | adjusted_low);
    }
}

static int resume_pair_outside(
        xdb_alien_biased_state XDB_NEAR *current,
        xdb_alien_biased_state XDB_NEAR *other)
{
    xdb_i32 x_delta = other->position_x - current->position_x;
    xdb_i32 z_delta = other->position_z - current->position_z;
    xdb_i16 y_delta = (xdb_i16)(
            (xdb_u16)other->position_y - (xdb_u16)current->position_y);

    if (x_delta >= -100 && x_delta <= 100
            && z_delta >= -200 && z_delta <= 200
            && y_delta >= -200 && y_delta < 200) {
        return 0;
    }

    {
        xdb_i16 steering = (xdb_i16)(
                (xdb_u16)current->field_04e
                + (xdb_u16)(-sar16(y_delta, 3u)));
        current->field_04e = sar16(steering, 1u);
    }

    {
        xdb_u16 sample_offset = current->field_050 & 0x0ffcu;
        volatile xdb_alien_trig_sample XDB_NEAR *sample =
                &xdb_alien_angle_table[sample_offset >> 2];
        xdb_i32 steering =
                (xdb_i32)sample->sine * x_delta
                - (xdb_i32)sample->cosine * z_delta;
        xdb_u16 step = steering < 0 ? 0xffe0u : 0x0010u;

        if (steering < 0) {
            steering = -steering;
        }
        (void)steering;
        current->field_050 = (xdb_u16)(sample_offset + step);
    }
    return 1;
}

static void XDB_NEAR resume_slot3_reset_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 ring_cursor;

    (void)context;
    ring_cursor = state->ring_offset;
    xdb_amer_slot3_ring[ring_cursor >> 3].field_000 = 0;
    xdb_amer_slot3_ring[ring_cursor >> 3].field_002 = 0;
    xdb_amer_slot3_ring[ring_cursor >> 3].field_004 = 0;
    xdb_amer_slot3_ring[ring_cursor >> 3].field_006 = 0;
}

static void XDB_NEAR resume_slot3_ring_zero_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);

static void XDB_NEAR resume_stage_1ccf(
        xdb_alien_method_context XDB_NEAR *context);

static void XDB_NEAR resume_slot3_ring_zero_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 ring_cursor;

    (void)context;
    if (xdb_amer_slot3_timer != 0u) {
        return;
    }
    ring_cursor = (xdb_u16)((state->ring_offset + 8u) & 0x03fcu);
    state->ring_offset = ring_cursor;
    xdb_amer_slot3_ring[ring_cursor >> 3].field_000 = 0;
    xdb_amer_slot3_ring[ring_cursor >> 3].field_002 = 0;
    xdb_amer_slot3_ring[ring_cursor >> 3].field_004 = 0;
    xdb_amer_slot3_ring[ring_cursor >> 3].field_006 = 0;
}

static void XDB_NEAR resume_stage_1cbf(
        xdb_alien_method_context XDB_NEAR *context)
{
    resume_apply_object_delta(context);
    --xdb_amer_slot3_timer;
    if ((xdb_i16)xdb_amer_slot3_timer >= 0) {
        return;
    }
    context->control.resume = resume_stage_1ccf;
}

static void XDB_NEAR resume_stage_1c7d(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_biased_state XDB_NEAR *current =
            (xdb_alien_biased_state XDB_NEAR *)
            ((xdb_u8 XDB_NEAR *)context->state + XDB_ALIEN_CURSOR_BIAS);
    xdb_alien_biased_state XDB_NEAR *other =
            (xdb_alien_biased_state XDB_NEAR *)(size_t)
            context->continuation.resume_state.value;

    resume_apply_object_delta(context);
    {
        xdb_i16 average = (xdb_i16)(
                (xdb_u16)other->field_054 + (xdb_u16)current->field_054);
        average = (xdb_i16)(
                (xdb_u16)average + (xdb_u16)sar16(other->field_054, 1u));
        current->field_054 = sar16(average, 1u);
    }
    if (!resume_pair_outside(current, other)) {
        return;
    }

    context->control.resume = resume_stage_1cbf;
    current->field_054 = 0;
    other->callback = xdb_amer_slot3_resume_callback;
    context->continuation.resume_state.value = (xdb_u16)(size_t)other;
    xdb_amer_slot3_timer = 0x18;
}

static void XDB_NEAR resume_stage_1ccf(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_alien_biased_state XDB_NEAR *current =
            (xdb_alien_biased_state XDB_NEAR *)
            ((xdb_u8 XDB_NEAR *)context->state + XDB_ALIEN_CURSOR_BIAS);
    xdb_alien_biased_state XDB_NEAR *other =
            (xdb_alien_biased_state XDB_NEAR *)(size_t)
            xdb_amer_slot11_cursor;

    current->field_054 = 0x64;
    if (!resume_pair_outside(current, other)) {
        return;
    }

    context->control.resume = xdb_amer_resume_1c34;
    other->callback = resume_slot3_reset_callback;
    current->field_054 = 0;
}

void XDB_NEAR xdb_amer_slot3_resume_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 ring_cursor = state->ring_offset;

    (void)context;
    state->position_x = 0;
    state->position_y = 0x06a4L;
    state->position_z = 0;
    state->field_04e = 0;
    state->field_050 = 0;
    state->field_052 = 0;
    state->field_054 = 0;
    state->callback = resume_slot3_ring_zero_callback;
    xdb_amer_slot3_ring[ring_cursor >> 3].field_000 = 0;
    xdb_amer_slot3_ring[ring_cursor >> 3].field_002 = 0;
    xdb_amer_slot3_ring[ring_cursor >> 3].field_004 = 0;
    xdb_amer_slot3_ring[ring_cursor >> 3].field_006 = 2;
}

void XDB_NEAR xdb_amer_resume_1c34(
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 queue_cursor = xdb_amer_slot11_queue_cursor;
    xdb_u16 queued_state =
            xdb_amer_slot11_state_queue[queue_cursor >> 1];

    if (queued_state == 0u) {
        queue_cursor = (xdb_u16)((queue_cursor + 2u) & 0x000fu);
        xdb_amer_slot11_queue_cursor = queue_cursor;
        context->state->field_0ac = (xdb_i16)(
                ((xdb_u16)context->state->field_0ac + 0x07e0u
                 & 0x0ffcu) - 0x0800u);
        context->state->field_0ae = (xdb_i16)(
                (xdb_u16)context->state->field_0ae + 8u);
        return;
    }

    xdb_amer_slot11_current_state = 0;
    xdb_amer_slot11_state_queue[queue_cursor >> 1] = 0;
    context->control.resume = resume_stage_1c7d;
    context->continuation.resume_state.value = queued_state;
}
