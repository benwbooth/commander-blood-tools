#include "../include/xdb_alien.h"

static xdb_u16 scrut_ror3_sbb_zero(xdb_u16 value)
{
    xdb_u16 rotated = (xdb_u16)((value >> 3) | (value << 13));

    return (xdb_u16)(rotated - ((value >> 2) & 1u));
}

static xdb_u16 scrut_ror7_sbb_zero(xdb_u16 value)
{
    xdb_u16 rotated = (xdb_u16)((value >> 7) | (value << 9));

    return (xdb_u16)(rotated - ((value >> 6) & 1u));
}

static xdb_i16 scrut_sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

static xdb_i16 scrut_clamp_height(xdb_i16 value)
{
    if (value < (xdb_i16)0xfd00u) {
        return (xdb_i16)0xfd00u;
    }
    if (value > 0x0300) {
        return 0x0300;
    }
    return value;
}

static xdb_i32 scrut_steering_score(
        const xdb_alien_biased_state XDB_NEAR *state,
        xdb_i32 first_factor)
{
    return first_factor * state->field_01a
            + (xdb_i32)state->field_038 * state->field_032;
}

static void XDB_NEAR scrut_slot2_fade_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context);

static void XDB_NEAR scrut_slot2_reset(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 random_value = scrut_ror7_sbb_zero(
            context->continuation.scrut_slot2.random_value);

    state->field_052 = 0;
    state->field_054 = 0x3c;
    state->field_05c = 0;
    state->callback = scrut_slot2_fade_callback;
    context->continuation.scrut_slot2.signed_seed =
            (xdb_i32)(xdb_i16)random_value;
    context->continuation.scrut_slot2.random_value = random_value;
    state->field_056 = 0x20;
}

static void XDB_NEAR scrut_slot2_selection_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 value = state->field_040;
    xdb_i32 score;

    if ((xdb_scrut_slot1_selection_state & 1u) == 0u) {
        state->callback = xdb_scrut_slot2_update;
        state->field_058 = 0x14;
        return;
    }
    if (value > 0x02bc || state->field_038 > 0x01f4
            || state->field_038 < (xdb_i16)0xfe0cu) {
        scrut_slot2_reset(state, context);
        return;
    }

    score = scrut_steering_score(state, -(xdb_i32)value);
    state->field_050 = (xdb_u16)(state->field_050
            + (score < 0 ? 0x0040u : 0xffc0u));
    state->field_052 = 0;
    state->field_04e = scrut_clamp_height(scrut_sar16((xdb_i16)(
            (xdb_u16)state->position_y + (xdb_u16)xdb_alien_view_y
            + (xdb_u16)state->field_04e), 1u));
}

static void XDB_NEAR scrut_slot2_selection_wait(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    (void)context;
    state->field_058 = 0x28;
    state->callback = scrut_slot2_selection_callback;
    if ((xdb_scrut_slot1_selection_state & 1u) == 0u) {
        state->callback = xdb_scrut_slot2_update;
        state->field_058 = 0x14;
    }
}

static void XDB_NEAR scrut_slot2_fade_callback(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 value = (xdb_i16)((xdb_u16)state->field_058
            - (xdb_u16)state->field_054);

    state->field_054 = (xdb_i16)((xdb_u16)state->field_054
            + (xdb_u16)scrut_sar16(value, 3u));
    if (context->continuation.scrut_slot2.duration != 0u) {
        --context->continuation.scrut_slot2.duration;
        return;
    }
    state->callback = xdb_scrut_slot2_update;
}

void XDB_NEAR xdb_scrut_slot2_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 numerator;
    xdb_i16 denominator;
    xdb_u16 random_value;
    xdb_i16 value;

    if ((xdb_scrut_slot1_selection_state & 3u) != 0u) {
        state->callback = scrut_slot2_selection_wait;
        context->continuation.scrut_slot2.duration = 0;
        return;
    }
    if (context->continuation.scrut_slot2.duration != 0u) {
        --context->continuation.scrut_slot2.duration;
        value = (xdb_i16)((xdb_u16)state->field_058
                - (xdb_u16)state->field_054);
        state->field_054 = (xdb_i16)((xdb_u16)state->field_054
                + (xdb_u16)scrut_sar16(value, 3u));
        return;
    }
    if (state->field_040 > 0x05dc
            || state->field_040 < (xdb_i16)0xfc18u
            || state->field_038 > 0x05dc
            || state->field_038 < (xdb_i16)0xfa24u) {
        scrut_slot2_reset(state, context);
        return;
    }

    random_value = scrut_ror3_sbb_zero(
            context->continuation.scrut_slot2.random_value);
    numerator = (xdb_i16)((random_value & 0x07ffu) - 0x03ffu);
    denominator = numerator;
    if (denominator < 0) {
        denominator = (xdb_i16)(-denominator);
    }
    denominator = (xdb_i16)((xdb_u16)denominator / 8u + 0x20u);
    context->continuation.scrut_slot2.duration = (xdb_u16)denominator;
    numerator = (xdb_i16)((xdb_u16)numerator
            - (xdb_u16)state->field_052);
    context->continuation.scrut_slot2.random_value = random_value;
    context->continuation.scrut_slot2.signed_seed =
            (xdb_i32)(xdb_i16)(numerator / denominator);
    state->field_058 = (xdb_u16)((0x0400u
            - (xdb_u16)(denominator - 0x20)) >> 4);
    state->field_054 = (xdb_i16)((xdb_u16)state->field_054
            + (xdb_u16)scrut_sar16((xdb_i16)((xdb_u16)state->field_058
            - (xdb_u16)state->field_054), 3u));

    if (xdb_alien_control_latch == (xdb_u16)(size_t)context) {
        state->field_04e = (xdb_i16)((xdb_u16)state->field_04e - 0x1eu);
        context->continuation.scrut_slot2.duration = 0xb2;
        state->callback = scrut_slot2_fade_callback;
        return;
    }

    state->field_052 = (xdb_i16)((xdb_u16)state->field_052
            + (xdb_u16)(xdb_i16)context->continuation.scrut_slot2.signed_seed);
    state->field_050 = (xdb_u16)(state->field_050
            + (xdb_u16)scrut_sar16((xdb_i16)context->continuation.scrut_slot2.signed_seed, 5u));
    state->field_04e = scrut_clamp_height(scrut_sar16((xdb_i16)(
            (xdb_u16)state->field_03c + (xdb_u16)state->field_04e), 1u));
}
