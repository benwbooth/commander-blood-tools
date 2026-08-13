#include <conio.h>
#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;

typedef volatile u8 far *graphics_buffer_ptr;

extern volatile u8 main_loop_hud_refresh_enabled;
extern const u8 main_loop_hud_text[];
extern graphics_buffer_ptr graphics_screen_buffer_ds;

void far planar_ui_text_render_10row_ds(
        const u8 near *text, u16 x, u16 y, u8 color);
void far video_retrace_phase_wait(void);
void near main_loop_hud_refresh(void);

#pragma intrinsic(_fmemset)
#pragma aux planar_ui_text_render_10row_ds "planar_ui_text_render_10row_" \
        parm [si] [bx] [dx] [ax] modify exact []
#pragma aux video_retrace_phase_wait modify exact []
#pragma aux main_loop_hud_refresh modify exact [ax bx cx dx di]

#define HUD_CLEAR_COLUMNS 20u
#define HUD_CLEAR_ROWS 14u
#define PLANAR_ROW_BYTES 80u

void near main_loop_hud_refresh(void)
{
    u8 far *cursor;
    u8 columns_remaining;
    u8 rows_remaining;

    if ((main_loop_hud_refresh_enabled & 1u) == 0) {
        return;
    }

    outpw(0x03c4u, 0x0f02u);
    cursor = (u8 far *)graphics_screen_buffer_ds + 0x1d2eu;
    rows_remaining = HUD_CLEAR_ROWS;
    do {
        columns_remaining = HUD_CLEAR_COLUMNS;
        do {
            *cursor++ = 0;
        } while (--columns_remaining != 0);
        cursor += PLANAR_ROW_BYTES - HUD_CLEAR_COLUMNS;
    } while (--rows_remaining != 0);

    planar_ui_text_render_10row_ds(main_loop_hud_text, 0x0087u, 0x0060u, 0xe8u);
    video_retrace_phase_wait();
}
