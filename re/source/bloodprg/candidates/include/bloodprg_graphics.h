#ifndef BLOODPRG_GRAPHICS_H
#define BLOODPRG_GRAPHICS_H

#include "bloodprg_common.h"

extern volatile cb_u8 CB_FAR *graphics_work_surface; /* GS:0x0ABC */
extern volatile cb_u8 palette_dirty; /* DS:0x5B55 */
extern volatile cb_u8 live_palette[768]; /* DS:0x5251 */
extern volatile cb_u8 render_update_flag_2751; /* GS:0x2751 */
extern volatile cb_u8 CB_FAR *graphics_display_buffer; /* GS:0x5221 */
extern volatile cb_u8 CB_FAR *graphics_back_buffer; /* GS:0x5229 */
extern volatile cb_u32 render_state_5251_dwords[]; /* GS:0x5251 */
extern volatile cb_u32 render_state_5851_dwords[]; /* GS:0x5851 */
extern volatile cb_u16 graphics_band_top_row; /* GS:0x5239 */
extern volatile cb_u16 graphics_band_bottom_row; /* GS:0x523B */

typedef struct bloodprg_layout_offset_result {
    cb_u16 x;
    cb_u16 y;
} bloodprg_layout_offset_result;

bloodprg_layout_offset_result CB_FAR layout_offset_calc(cb_u16 columns,
        cb_u16 rows); /* 0x000E62 */
void CB_FAR composite_draw_a(cb_u8 color, cb_u16 x, cb_u16 y,
        cb_u16 width, cb_u16 height); /* 0x0299:0x0BB5 */
void CB_FAR blit_coord_guard_c(cb_u8 color, cb_u16 x, cb_u16 y,
        cb_u16 width, cb_u16 height); /* 0x0299:0x0CDC */
void CB_FAR video_retrace_wait(void); /* 0x0000:0x05D7 */
void CB_FAR vga_palette_write(const volatile cb_u8 *palette); /* 0x0299:0x0000 */
void CB_NEAR palette_upload_if_dirty(void); /* 0x00178B */

#endif
