#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_ship3d.h"

void CB_FAR ship_3d_hud_palette_snapshot_and_camera_reset(void)
{
    draw_hud_element_2bc7();
    _fmemcpy(
        (void CB_FAR *)ship_3d_hud_palette_snapshot,
        (const void CB_FAR *)(pbm_live_palette
            + SHIP_3D_HUD_PALETTE_FIRST * 3u),
        (cb_u16)sizeof(ship_3d_hud_palette_snapshot));
    ship_3d_camera_x = 10000;
    ship_3d_camera_y = 12000;
    ship_3d_camera_z = 0;
}
