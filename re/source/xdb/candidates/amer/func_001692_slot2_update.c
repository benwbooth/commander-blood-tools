#include "../include/xdb_alien.h"

static xdb_u16 ror3_sbb_zero(xdb_u16 value)
{
    xdb_u16 rotated = (xdb_u16)((value >> 3) | (value << 13));

    return (xdb_u16)(rotated - ((value >> 2) & 1u));
}

static xdb_u16 ror7_sbb_zero(xdb_u16 value)
{
    xdb_u16 rotated = (xdb_u16)((value >> 7) | (value << 9));

    return (xdb_u16)(rotated - ((value >> 6) & 1u));
}

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

static xdb_i16 clamp_camera_height(xdb_i16 value)
{
    if (value < (xdb_i16)0xfd00u) {
        return (xdb_i16)0xfd00u;
    }
    if (value > 0x0300) {
        return 0x0300;
    }
    return value;
}

static xdb_i16 clamp_state_height(
        const xdb_alien_biased_state XDB_NEAR *state)
{
    xdb_i16 value = (xdb_i16)(
            (xdb_u16)state->position_y + (xdb_u16)xdb_alien_view_y
            + (xdb_u16)state->field_04e);

    return clamp_camera_height(sar16(value, 1u));
}

static volatile xdb_i16 XDB_NEAR *state_word(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_u16 offset)
{
    return (volatile xdb_i16 XDB_NEAR *)
            ((volatile xdb_u8 XDB_NEAR *)state + offset);
}

static volatile xdb_u16 XDB_NEAR *context_word(
        xdb_alien_method_context XDB_NEAR *context,
        xdb_u16 offset)
{
    return (volatile xdb_u16 XDB_NEAR *)
            ((volatile xdb_u8 XDB_NEAR *)context + offset);
}

static xdb_i32 steering_score(
        const xdb_alien_biased_state XDB_NEAR *state,
        xdb_i32 first_factor)
{
    return first_factor * state->field_01a
            + (xdb_i32)state->field_038 * state->field_032;
}

static void XDB_NEAR slot2_invalid_reset(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 random_value = ror7_sbb_zero(
            context->continuation.amer_slot2.random_value);

    state->field_052 = 0;
    context->continuation.amer_slot2_motion.velocity_x = 0;
    state->field_05c = 0;
    state->field_054 = 0x3c;
    state->callback = xdb_amer_slot2_steer_update;
    context->continuation.amer_slot2_motion.velocity_x =
            sar16((xdb_i16)random_value, 6u);
    context->continuation.amer_slot2.random_value = random_value;
    state->field_056 = 0x20;
}

static void XDB_NEAR slot2_selection_late_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);

static void XDB_NEAR slot2_selection_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i32 first_factor;
    xdb_i32 score;
    xdb_i16 value;

    if ((xdb_amer_slot1_selection_state & 1u) == 0u) {
        state->callback = xdb_amer_slot2_update;
        state->field_058 = 0x14;
        return;
    }

    value = state->field_040;
    if (value > 0x0bb8 || state->field_038 > 0x03e8
            || state->field_038 < (xdb_i16)0xfc18u) {
        slot2_invalid_reset(state, context);
        return;
    }
    if (value < 0x0320) {
        state->field_058 = 0x50;
        state->callback = slot2_selection_late_callback;
        return;
    }

    first_factor = -(xdb_i32)value;
    score = steering_score(state, first_factor);
    state->field_050 = (xdb_u16)(state->field_050
            + (score < 0 ? 0x0040u : 0xffc0u));
    state->field_052 = 0;
    state->field_04e = clamp_state_height(state);
}

void XDB_NEAR xdb_amer_slot2_selection_wait(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    (void)context;
    state->field_058 = 0x28;
    state->callback = slot2_selection_callback;
    if ((xdb_amer_slot1_selection_state & 1u) == 0u) {
        state->callback = xdb_amer_slot2_update;
        state->field_058 = 0x14;
    }
}

static void XDB_NEAR slot2_selection_late_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i32 first_factor;
    xdb_i32 score;
    xdb_i16 value;

    value = state->field_040;
    if (value > 0x03e8 || state->field_038 > 0x01f4
            || state->field_038 < (xdb_i16)0xfe0cu) {
        slot2_invalid_reset(state, context);
        return;
    }

    first_factor = -(xdb_i32)(value - 0x00c8);
    score = steering_score(state, first_factor);
    context->continuation.amer_slot2_motion.velocity_x =
            (xdb_i16)(score < 0 ? 0x0030 : (xdb_i16)0xffd0u);
    state->field_04e = clamp_state_height(state);
}

void XDB_NEAR xdb_amer_slot2_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 countdown;
    xdb_i16 value;

    if (xdb_alien_method_delta < 0) {
        goto countdown_step;
    }
    if ((xdb_amer_slot1_selection_state & 1u) != 0u) {
        state->field_058 = 0x28;
        state->callback = xdb_amer_slot2_selection_wait;
        return;
    }

countdown_step:
    countdown = (xdb_i16)((xdb_u16)
            context->continuation.amer_slot2.field_038 - 1u);
    context->continuation.amer_slot2.field_038 = countdown;
    if (countdown >= 0) {
        value = (xdb_i16)((xdb_u16)state->field_058
                - (xdb_u16)state->field_054);
        state->field_054 = (xdb_i16)(
                (xdb_u16)state->field_054 + (xdb_u16)sar16(value, 3u));
        return;
    }

    value = state->field_040;
    if (value > 0x05dc || value < (xdb_i16)0xfc18u
            || state->field_038 > 0x05dc
            || state->field_038 < (xdb_i16)0xfa24u) {
        slot2_invalid_reset(state, context);
        return;
    }

    {
        xdb_u16 random_value = ror3_sbb_zero(
                context->continuation.amer_slot2.random_value);
        xdb_i16 numerator = (xdb_i16)((random_value & 0x07ffu) - 0x03ffu);
        xdb_i16 denominator = numerator;

        if (denominator < 0) {
            denominator = (xdb_i16)(-denominator);
        }
        denominator = (xdb_i16)(((xdb_u16)denominator >> 2) + 0x10u);
        context->continuation.amer_slot2.field_038 = denominator;
        numerator = (xdb_i16)(numerator - state->field_052);
        context->continuation.amer_slot2.random_value = random_value;
        context->continuation.amer_slot2_motion.velocity_x =
                (xdb_i16)(numerator / denominator);
        state->field_058 = 0x14;
    }

    state->field_04e = clamp_camera_height(sar16((xdb_i16)(
            (xdb_u16)state->field_03c + (xdb_u16)state->field_04e), 1u));

    value = (xdb_i16)((xdb_u16)state->field_058
            - (xdb_u16)state->field_054);
    state->field_054 = (xdb_i16)(
            (xdb_u16)state->field_054 + (xdb_u16)sar16(value, 3u));
    if (state->field_038 >= (xdb_i16)0xffd8u
            && state->field_038 < 0x28
            && state->field_03c >= (xdb_i16)0xffd8u
            && state->field_03c <= 0x28
            && (xdb_i16)state->field_040 <= 0x28
            && (xdb_i16)state->field_040 > (xdb_i16)0xffb0u) {
        goto camera_launch;
    }

    value = (xdb_i16)((xdb_u16)
            context->continuation.amer_slot2_motion.velocity_x
            + (xdb_u16)state->field_052);
    state->field_052 = value;
    state->field_050 = (xdb_u16)(state->field_050
            + (xdb_u16)sar16(value, 3u));
    *context_word(context, 0x42) = (xdb_u16)(
            (*context_word(context, 0x42) + 0x84u) & 0x03ffu);

    {
        xdb_i16 phase = (xdb_i16)(state->field_052 + 0x20);

        if (phase < 0) {
            *state_word(state, 0x0acu) = (xdb_i16)0xff00u;
            *state_word(state, 0x0b0u) = (xdb_i16)state->position_x;
            *state_word(state, 0x10au) = 0x0100;
            *state_word(state, 0x10eu) = (xdb_i16)(-state->position_x);
            *state_word(state, 0x168u) = (xdb_i16)0xff00u;
            *state_word(state, 0x16cu) = (xdb_i16)(-state->position_x);
            *state_word(state, 0x1c6u) = 0x0100;
            *state_word(state, 0x1cau) = state->position_x;
            return;
        }
        if (phase < 0x40) {
            *state_word(state, 0x0acu) = (xdb_i16)0xfe00u;
            *state_word(state, 0x0b0u) = 0;
            *state_word(state, 0x10au) = 0x0200;
            *state_word(state, 0x10eu) = 0;
            *state_word(state, 0x168u) = (xdb_i16)0xff00u;
            *state_word(state, 0x16cu) = (xdb_i16)(-state->position_x);
            *state_word(state, 0x1c6u) = 0x0100;
            *state_word(state, 0x1cau) = (xdb_i16)(-state->position_x);
            return;
        }
        *state_word(state, 0x0acu) = (xdb_i16)0xff00u;
        *state_word(state, 0x0b0u) = (xdb_i16)state->position_x;
        *state_word(state, 0x10au) = 0x0100;
        *state_word(state, 0x10eu) = (xdb_i16)(-state->position_x);
        *state_word(state, 0x168u) = (xdb_i16)0xfe00u;
        *state_word(state, 0x16cu) = 0;
        *state_word(state, 0x1c6u) = 0x0200;
        *state_word(state, 0x1cau) = 0;
    }
    return;

camera_launch:
    if ((xdb_i16)state->field_040 < 0) {
        state->field_050 = (xdb_u16)(xdb_alien_camera_pan + 0x0800u)
                & 0x0ffcu;
        return;
    }

    state->field_054 = 0;
    state->position_x = -(xdb_i32)(
            ((xdb_i32)xdb_alien_camera_matrix[6] * -0x00a0
             + xdb_alien_camera_position[0]) >> 16);
    state->position_y = -(xdb_i32)(
            ((xdb_i32)xdb_alien_camera_matrix[7] * -0x00a0
             + xdb_alien_camera_position[1]) >> 16);
    state->position_z = -(xdb_i32)(
            ((xdb_i32)xdb_alien_camera_matrix[8] * -0x00a0
             + xdb_alien_camera_position[2]) >> 16);

    {
        xdb_i16 depth = sar16(xdb_alien_camera_depth_step, 1u);
        xdb_i16 scale;

        if (depth < 0x28) {
            depth = 0x28;
        }
        scale = (xdb_i16)((xdb_u16)depth >> 2);
        scale = (xdb_i16)(scale + 0x14);
        context->continuation.amer_slot2_motion.countdown = depth;
        context->continuation.amer_slot2_motion.velocity_x =
                (xdb_i16)(((xdb_i32)xdb_alien_camera_matrix[6] * scale) >> 18);
        context->continuation.amer_slot2_motion.velocity_y =
                (xdb_i16)(((xdb_i32)xdb_alien_camera_matrix[7] * scale) >> 18);
        context->continuation.amer_slot2_motion.velocity_z =
                (xdb_i16)(((xdb_i32)xdb_alien_camera_matrix[8] * scale) >> 18);
    }
    xdb_alien_camera_depth_step = (xdb_i16)0xffc0u;
    context->control.state = (xdb_i16)0x8001u;
    state->callback = xdb_amer_slot2_return_update;
    xdb_amer_slot2_active = 1;
    xdb_alien_callback_countdown = 1;
}
