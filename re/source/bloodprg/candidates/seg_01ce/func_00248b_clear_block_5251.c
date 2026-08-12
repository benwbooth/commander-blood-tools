#include "../include/bloodprg_ship3d.h"

void CB_FAR clear_block_5251(void)
{
    cb_u16 i;

    for (i = 0; i < 0x90u; ++i) {
        ship_3d_render_state_block[i] = 0;
    }
}
