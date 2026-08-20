#include "../include/xdb_alien.h"

static xdb_u16 ror3_sbb_zero(xdb_u16 value)
{
    xdb_u16 rotated = (xdb_u16)((value >> 3) | (value << 13));

    return (xdb_u16)(rotated - ((value >> 2) & 1u));
}

void XDB_NEAR xdb_croolis_slot2_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 countdown;
    xdb_i16 numerator;
    xdb_i16 absolute;
    xdb_i16 denominator;
    xdb_u16 random_value;

    if ((xdb_croolis_slot1_selection_state & 3u) != 0u) {
        xdb_croolis_slot2_selection_init(state, context);
        return;
    }

    countdown = (xdb_i16)(context->continuation.croolis_slot2.duration - 1);
    context->continuation.croolis_slot2.duration = countdown;
    if (countdown >= 0) {
        xdb_croolis_slot2_common_dispatch(state, context);
        return;
    }

    if ((xdb_u16)(state->field_040 + 0x01f4u) > 0x0bb8u
            || (xdb_u16)((xdb_u16)state->field_038 + 0x03e8u)
                    > 0x07d0u) {
        xdb_croolis_slot2_reset_or_camera(state, context, 0x07d0L);
        return;
    }

    random_value = ror3_sbb_zero(
            context->continuation.croolis_slot2.random_value);
    numerator = (xdb_i16)((random_value & 0x03ffu) - 0x01ffu);
    absolute = numerator;
    if (absolute < 0) {
        absolute = (xdb_i16)-absolute;
    }
    denominator = (xdb_i16)(((xdb_u16)absolute >> 1) + 0x10u);
    context->continuation.croolis_slot2.duration = denominator;
    state->field_058 = (xdb_u16)((0x0300u - (xdb_u16)absolute) >> 3);
    numerator = (xdb_i16)((xdb_u16)numerator
            - (xdb_u16)state->field_052);
    context->continuation.croolis_slot2.random_value = random_value;
    context->continuation.croolis_slot2.field_03a =
            (xdb_i16)(numerator / denominator);
    xdb_croolis_slot2_common_dispatch(state, context);
}
