#include "../include/bloodprg_graphics.h"

int CB_NEAR gfx_scanline_advance(bloodprg_gfx_scanline_state *state)
{
    --state->rows_remaining;
    if (state->rows_remaining == 0) {
        return 0;
    }

    state->row_offset = (cb_u16)(state->row_offset + 320u);
    return 1;
}
