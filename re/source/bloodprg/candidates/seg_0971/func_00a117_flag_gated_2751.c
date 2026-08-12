#include "../include/bloodprg_graphics.h"

void CB_NEAR flag_gated_2751(void)
{
    cb_u16 index;

    if (render_update_flag_2751 & 1u) {
        return;
    }

    for (index = 0; index != 0x60u; ++index) {
        render_state_5851_dwords[index] = render_state_5251_dwords[index];
    }
}
