#include "../include/xdb_alien.h"

static xdb_u16 ror3_sbb_zero(xdb_u16 value)
{
    xdb_u16 rotated = (xdb_u16)((value >> 3) | (value << 13));

    return (xdb_u16)(rotated - ((value >> 2) & 1u));
}

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

static xdb_i16 motion_sample(xdb_u16 offset)
{
    return *(volatile xdb_i16 XDB_NEAR *)(xdb_alien_motion_samples + offset);
}

void XDB_NEAR xdb_croolis_slot1_motion_continuation(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_i16 denominator;
    xdb_i16 numerator;
    xdb_i16 sample;
    xdb_i16 distance;
    xdb_u16 random;
    xdb_u16 cursor;
    xdb_i16 step;
    xdb_i16 x_distance;
    xdb_i16 z_distance;

    (void)context;
    state->field_054 = 0x000c;
    state->field_010 = (xdb_i16)(state->field_010 - 1);
    if (state->field_010 >= 0) {
        goto advance;
    }

    random = state->field_05c;
    z_distance = (xdb_i16)(state->position_z + xdb_alien_view_z);
    distance = (xdb_i16)(state->field_050 + 0x0800);
    if (z_distance < -0x03e8) {
        goto fixed_step;
    }
    distance = (xdb_i16)(distance + 0x0800);
    if (z_distance > 0x03e8) {
        goto fixed_step;
    }

    x_distance = (xdb_i16)(state->position_x + xdb_alien_view_x);
    distance = (xdb_i16)(distance + 0x0400);
    if (x_distance < -0x03e8) {
        goto fixed_step;
    }
    distance = (xdb_i16)(distance + 0x0800);
    if (x_distance >= 0x03e8) {
        goto fixed_step;
    }

    random = ror3_sbb_zero(random);
    numerator = (xdb_i16)((random & 0x07ffu) - 0x03ffu);
    denominator = numerator;
    if (denominator < 0) {
        denominator = (xdb_i16)(-denominator);
    }
    denominator = (xdb_i16)(((xdb_u16)denominator >> 1) + 0x0010);
    state->field_010 = denominator;
    step = numerator;
    goto divide_step;

fixed_step:
    distance = (xdb_i16)((xdb_u16)distance & 0x0ffcu);
    denominator = 0x0020;
    state->field_010 = 0x0010;
    step = (xdb_i16)(-sar16((xdb_i16)(distance - 0x0800), 2u));

divide_step:
    numerator = (xdb_i16)(step - state->ring_offset);
    state->field_05c = random;
    state->field_056 = (xdb_i16)(numerator / denominator);

advance:
    distance = (xdb_i16)(state->field_056 + state->ring_offset);
    state->ring_offset = (xdb_u16)distance;
    state->field_050 = (xdb_u16)(state->field_050 + sar16(distance, 5u));
    cursor = (xdb_u16)((state->field_058 + 0x0080u) & 0x0ffcu);
    state->field_058 = cursor;
    sample = sar16(motion_sample(cursor), 5u);
    state->field_052 = (xdb_i16)(distance + sample);
}
