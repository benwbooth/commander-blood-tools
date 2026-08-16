#include "../include/xdb_alien.h"

static xdb_u16 croolis_ror3_sbb_zero(xdb_u16 value)
{
    xdb_u16 rotated = (xdb_u16)((value >> 3) | (value << 13));

    return (xdb_u16)(rotated - ((value >> 2) & 1u));
}

static xdb_u16 croolis_ror7_sbb_zero(xdb_u16 value)
{
    xdb_u16 rotated = (xdb_u16)((value >> 7) | (value << 9));

    return (xdb_u16)(rotated - ((value >> 6) & 1u));
}

static xdb_i16 croolis_sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

static xdb_i16 croolis_clamp_height(xdb_i16 value)
{
    if (value < (xdb_i16)0xfd00u) {
        return (xdb_i16)0xfd00u;
    }
    if (value > 0x0300) {
        return 0x0300;
    }
    return value;
}

static volatile xdb_i16 XDB_NEAR *croolis_state_word(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_u16 offset)
{
    return (volatile xdb_i16 XDB_NEAR *)
            ((volatile xdb_u8 XDB_NEAR *)state + offset);
}

static xdb_i32 croolis_steering_score(
        const xdb_alien_biased_state XDB_NEAR *state,
        xdb_i32 first_factor)
{
    return first_factor * state->field_01a
            + (xdb_i32)state->field_038 * state->field_032;
}

static void XDB_NEAR croolis_slot2_fade_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);

static void XDB_NEAR croolis_slot2_reset(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 random_value = croolis_ror7_sbb_zero(
            context->continuation.croolis_slot2.random_value);

    state->field_052 = 0;
    state->field_054 = 0x3c;
    state->field_05c = 0;
    state->callback = croolis_slot2_fade_callback;
    context->continuation.croolis_slot2.field_03a = 0;
    context->continuation.croolis_slot2.random_value = random_value;
    state->field_056 = 0x20;
}

static void croolis_slot2_object_motion(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context,
        xdb_i16 delta)
{
    volatile xdb_i16 XDB_FAR *object = XDB_FAR_AT(
            xdb_i16, xdb_alien_object_segment, context->object_offset);
    xdb_u16 count = context->object_count;

    while (count-- != 0u) {
        *object = (xdb_i16)((xdb_u16)*object + (xdb_u16)delta);
        object += 0x0au;
    }
    (void)state;
}

static void XDB_NEAR croolis_slot2_selection_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 value = state->field_040;
    xdb_i32 score;

    if ((xdb_croolis_slot1_selection_state & 1u) == 0u) {
        state->callback = xdb_croolis_slot2_update;
        state->field_058 = 0x14;
        return;
    }
    if (value > 0x0bb8 || state->field_038 > 0x03e8
            || state->field_038 < (xdb_i16)0xfc18u) {
        croolis_slot2_reset(state, context);
        return;
    }
    if (value < 0x0320) {
        state->field_058 = 0x50;
        state->callback = croolis_slot2_fade_callback;
        return;
    }

    score = croolis_steering_score(state, -(xdb_i32)value);
    state->field_050 = (xdb_u16)(state->field_050
            + (score < 0 ? 0x0040u : 0xffc0u));
    state->field_052 = 0;
    state->field_04e = croolis_clamp_height(croolis_sar16((xdb_i16)(
            (xdb_u16)state->position_y + (xdb_u16)xdb_alien_view_y
            + (xdb_u16)state->field_04e), 1u));
}

static void XDB_NEAR croolis_slot2_selection_wait(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    (void)context;
    state->field_058 = 0x28;
    state->callback = croolis_slot2_selection_callback;
    if ((xdb_croolis_slot1_selection_state & 1u) == 0u) {
        state->callback = xdb_croolis_slot2_update;
        state->field_058 = 0x14;
    }
}

static void XDB_NEAR croolis_slot2_fade_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 value = (xdb_i16)((xdb_u16)state->field_058
            - (xdb_u16)state->field_054);

    state->field_054 = (xdb_i16)((xdb_u16)state->field_054
            + (xdb_u16)croolis_sar16(value, 3u));
    if (context->continuation.croolis_slot2.duration != 0u) {
        --context->continuation.croolis_slot2.duration;
        return;
    }
    state->callback = xdb_croolis_slot2_update;
}

static void XDB_NEAR croolis_slot2_camera_launch(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 depth = croolis_sar16(xdb_alien_camera_depth_step, 1u);
    xdb_i16 scale;

    state->field_054 = 0;
    state->position_x = -(xdb_i32)(((xdb_i32)xdb_alien_camera_matrix[6]
            * -0x00a0 + xdb_alien_camera_position[0]) >> 16);
    state->position_y = -(xdb_i32)(((xdb_i32)xdb_alien_camera_matrix[7]
            * -0x00a0 + xdb_alien_camera_position[1]) >> 16);
    state->position_z = -(xdb_i32)(((xdb_i32)xdb_alien_camera_matrix[8]
            * -0x00a0 + xdb_alien_camera_position[2]) >> 16);
    if (depth < 0x28) {
        depth = 0x28;
    }
    scale = (xdb_i16)((xdb_u16)depth >> 2);
    scale = (xdb_i16)(scale + 0x14);
    context->control.state = (xdb_i16)0x8001u;
    context->continuation.croolis_slot2.duration = (xdb_u16)depth;
    context->continuation.croolis_slot2.field_03a =
            (xdb_i16)(((xdb_i32)xdb_alien_camera_matrix[6] * scale) >> 18);
    state->callback = xdb_croolis_slot2_update;
    xdb_alien_camera_depth_step = (xdb_i16)0xffc0u;
    xdb_alien_callback_countdown = 1;
}

void XDB_NEAR xdb_croolis_slot2_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 value;
    xdb_i16 denominator;
    xdb_i16 numerator;
    xdb_u16 random_value;

    if ((xdb_croolis_slot1_selection_state & 3u) != 0u) {
        state->callback = croolis_slot2_selection_wait;
        context->continuation.croolis_slot2.duration = 0;
        return;
    }

    if (context->continuation.croolis_slot2.duration != 0u) {
        --context->continuation.croolis_slot2.duration;
        value = (xdb_i16)((xdb_u16)state->field_058
                - (xdb_u16)state->field_054);
        state->field_054 = (xdb_i16)((xdb_u16)state->field_054
                + (xdb_u16)croolis_sar16(value, 3u));
        return;
    }

    if (state->field_040 > 0x05dc
            || state->field_040 < (xdb_i16)0xfc18u
            || state->field_038 > 0x05dc
            || state->field_038 < (xdb_i16)0xfa24u) {
        croolis_slot2_reset(state, context);
        return;
    }

    random_value = croolis_ror3_sbb_zero(
            context->continuation.croolis_slot2.random_value);
    numerator = (xdb_i16)((random_value & 0x03ffu) - 0x01ffu);
    denominator = numerator;
    if (denominator < 0) {
        denominator = (xdb_i16)(-denominator);
    }
    denominator = (xdb_i16)((xdb_u16)denominator / 2u + 0x10u);
    context->continuation.croolis_slot2.duration = (xdb_u16)denominator;
    numerator = (xdb_i16)((xdb_u16)numerator
            - (xdb_u16)state->field_052);
    context->continuation.croolis_slot2.random_value = random_value;
    context->continuation.croolis_slot2.field_03a =
            (xdb_i16)(numerator / denominator);
    state->field_058 = (xdb_u16)((0x0300u
            - (xdb_u16)(denominator - 0x10)) >> 3);
    state->field_054 = (xdb_i16)((xdb_u16)state->field_054
            + (xdb_u16)croolis_sar16((xdb_i16)((xdb_u16)state->field_058
            - (xdb_u16)state->field_054), 3u));

    if (xdb_alien_control_latch == (xdb_u16)(size_t)context) {
        state->field_04e = (xdb_i16)((xdb_u16)state->field_04e - 0x1eu);
        context->continuation.croolis_slot2.duration = 0xb2;
        state->callback = croolis_slot2_fade_callback;
        return;
    }

    state->field_052 = (xdb_i16)((xdb_u16)state->field_052
            + (xdb_u16)context->continuation.croolis_slot2.field_03a);
    state->field_050 = (xdb_u16)(state->field_050
            + (xdb_u16)croolis_sar16(context->continuation.croolis_slot2.field_03a, 4u));
    state->field_04e = croolis_clamp_height(croolis_sar16((xdb_i16)(
            (xdb_u16)state->field_03c + (xdb_u16)state->field_04e), 1u));
    croolis_slot2_object_motion(state, context,
            context->continuation.croolis_slot2.field_03a);

    value = (xdb_i16)((xdb_u16)state->field_040
            + (xdb_u16)xdb_alien_view_y);
    if (value >= 0) {
        croolis_slot2_camera_launch(state, context);
    }
}
