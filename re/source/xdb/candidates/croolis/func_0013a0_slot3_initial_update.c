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

static xdb_i16 sar16_with_carry(xdb_i16 value, unsigned shift, xdb_u16 *carry)
{
    xdb_u16 bits = (xdb_u16)value;

    *carry = 0;
    while (shift-- != 0u) {
        *carry = (xdb_u16)(bits & 1u);
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

void XDB_NEAR xdb_croolis_slot3_initial_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 ring_cursor;
    xdb_u16 ring_index;
    xdb_u16 ax;
    xdb_u16 bx;
    xdb_u16 cx;
    xdb_u16 dx;

    (void)context;
    ring_cursor = state->ring_offset;
    ring_index = (xdb_u16)(ring_cursor >> 3);
    ax = (xdb_u16)xdb_croolis_slot3_ring[ring_index].field_000;
    bx = (xdb_u16)xdb_croolis_slot3_ring[ring_index].field_002;
    dx = (xdb_u16)xdb_croolis_slot3_ring[ring_index].field_004;
    xdb_croolis_slot3_ring[ring_index].field_006 = 0;
    state->field_04e = (xdb_i16)(state->field_04e + (xdb_i16)ax);
    state->field_050 = (xdb_u16)(state->field_050 + bx);
    state->field_054 = (xdb_i16)dx;
    if (xdb_croolis_slot3_timer != 0u) {
        return;
    }

    ring_cursor = (xdb_u16)((state->ring_offset + 8u) & 0x03fcu);
    state->ring_offset = ring_cursor;
    if (--state->field_056 < 0) {
        xdb_u16 random_a;
        xdb_u16 random_b;
        xdb_i16 numerator;
        xdb_u16 carry;

        random_a = ror3_sbb_zero((xdb_u16)state->field_05c);
        cx = random_a;
        random_a = ror3_sbb_zero(random_a);
        cx = (xdb_u16)((cx & 0x003fu) + 8u);
        random_b = random_a;
        xdb_croolis_slot3_ring[ring_cursor >> 3].field_002 =
                sar16((xdb_i16)random_a, 9u);

        dx = (xdb_u16)((state->field_04e + 0x0800) & 0x0ffcu);
        dx = (xdb_u16)(dx - 0x0800u);
        state->field_04e = (xdb_i16)dx;
        dx = (xdb_u16)(0u - dx);

        ax = ror3_sbb_zero(random_b);
        random_b = ax;
        ax = (xdb_u16)(ax & 0x0ffcu);
        ax = (xdb_u16)(ax - 0x0800u);
        ax = (xdb_u16)sar16_with_carry((xdb_i16)ax, 2u, &carry);
        numerator = (xdb_i16)(ax + dx + carry);
        xdb_croolis_slot3_ring[ring_cursor >> 3].field_000 =
                (xdb_i16)(numerator / (xdb_i16)cx);
        state->field_056 = sar16((xdb_i16)cx, 3u);
        ax = ror3_sbb_zero(random_b);
        state->field_05c = ax;
        xdb_croolis_slot3_ring[ring_cursor >> 3].field_004 =
                (xdb_i16)((ax & 0x007fu) + 8u);
        return;
    }

    ring_index = (xdb_u16)(ring_cursor >> 3);
    xdb_croolis_slot3_ring[ring_index].field_000 = (xdb_i16)ax;
    xdb_croolis_slot3_ring[ring_index].field_002 = (xdb_i16)bx;
    xdb_croolis_slot3_ring[ring_index].field_004 = (xdb_i16)dx;
}
