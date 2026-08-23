#include "../include/xdb_alien.h"

static xdb_i16 sar16(xdb_i16 value, unsigned shift)
{
    xdb_u16 bits = (xdb_u16)value;

    while (shift-- != 0u) {
        bits = (xdb_u16)((bits >> 1) | (bits & 0x8000u));
    }
    return (xdb_i16)bits;
}

void XDB_NEAR xdb_croolis_slot1_camera_update(
        xdb_alien_biased_state XDB_NEAR *state,
        xdb_alien_method_context XDB_NEAR *context)
{
    xdb_u16 pan;
    xdb_u16 current;

    (void)context;
    pan = (xdb_u16)xdb_alien_camera_pan & 0x0ffcu;
    current = state->field_050 & 0x0ffcu;
    state->field_056 = sar16((xdb_i16)(pan - current), 4u);
    state->field_010 = sar16(state->field_052, 4u);
    state->callback = xdb_croolis_slot1_motion_update;
}
