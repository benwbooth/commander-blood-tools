#include "../include/bloodprg_graphics.h"

#include <string.h>

void CB_NEAR flag_gated_2751(void)
{
    if (render_update_flag_2751 & 1u) {
        return;
    }

    memcpy(palette_low_5851_dwords, palette_low_5251_dwords, 384u);
}
