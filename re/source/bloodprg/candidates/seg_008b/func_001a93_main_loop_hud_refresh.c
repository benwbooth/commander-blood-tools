#include <conio.h>

#include "../include/bloodprg_graphics.h"

#define BLOODPRG_HUD_CLEAR_COLUMNS 20u
#define BLOODPRG_HUD_CLEAR_ROWS 14u
#define BLOODPRG_PLANAR_ROW_BYTES 80u

void CB_NEAR main_loop_hud_refresh(void)
{
    cb_u8 CB_FAR *cursor;
    cb_u8 columns_remaining;
    cb_u8 rows_remaining;

    if ((main_loop_hud_refresh_enabled & 1u) == 0) {
        return;
    }

    outpw(0x03c4u, 0x0f02u);
    cursor = (cb_u8 CB_FAR *)graphics_screen_buffer_ds + 0x1d2eu;
    rows_remaining = BLOODPRG_HUD_CLEAR_ROWS;
    do {
        columns_remaining = BLOODPRG_HUD_CLEAR_COLUMNS;
        do {
            *cursor++ = 0;
        } while (--columns_remaining != 0);
        cursor += BLOODPRG_PLANAR_ROW_BYTES - BLOODPRG_HUD_CLEAR_COLUMNS;
    } while (--rows_remaining != 0);

    planar_ui_text_render_10row_ds(
            main_loop_hud_text, 0x0087u, 0x0060u, 0xe8u);
    video_retrace_phase_wait();
}
