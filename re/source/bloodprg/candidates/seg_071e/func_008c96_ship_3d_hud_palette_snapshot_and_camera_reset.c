#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_ship3d.h"

void CB_FAR ship_3d_hud_palette_snapshot_and_camera_reset(void)
{
#if defined(__TURBOC__) || defined(__BORLANDC__)
    asm push bp;
    asm push ax;
    asm push ds;
    asm push es;
    asm push di;
    asm push si;
    asm push cx;
    asm push bx;
    asm push dx;
#elif defined(__WATCOMC__)
    /* Watcom saves the allocated general registers but not AX or ES. */
    _asm push ax;
    _asm push es;
#endif

    draw_hud_element_2bc7();
#if defined(__TURBOC__) || defined(__BORLANDC__)
    asm pushf;
#elif defined(__WATCOMC__)
    _asm pushf;
#endif

    _fmemcpy(
        (void CB_FAR *)ship_3d_hud_palette_snapshot,
        (const void CB_FAR *)(pbm_live_palette
            + SHIP_3D_HUD_PALETTE_FIRST * 3u),
        (cb_u16)sizeof(ship_3d_hud_palette_snapshot));
    ship_3d_camera_x = 10000;
    ship_3d_camera_y = 12000;
    ship_3d_camera_z = 0;

#if defined(__TURBOC__) || defined(__BORLANDC__)
    asm popf;
    asm pop dx;
    asm pop bx;
    asm pop cx;
    asm pop si;
    asm pop di;
    asm pop es;
    asm pop ds;
    asm pop ax;
    asm pop bp;
#elif defined(__WATCOMC__)
    _asm popf;
    _asm pop es;
    _asm pop ax;
#endif
}
