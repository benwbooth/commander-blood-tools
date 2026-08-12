#ifndef BLOODPRG_GRAPHICS_H
#define BLOODPRG_GRAPHICS_H

#include "bloodprg_common.h"
#include "bloodprg_hardware.h"

typedef volatile cb_u8 CB_FAR *bloodprg_graphics_buffer_ptr;

extern bloodprg_graphics_buffer_ptr CB_GAME_DATA
        graphics_work_surface; /* GS:0x0ABC */
extern volatile cb_u8 palette_dirty; /* game data:0x5B55 */
extern volatile cb_u8 live_palette[768]; /* game data:0x5251 */
/* First 192 RGB entries of live_palette, addressed through GS by 0x00248B. */
extern cb_u32 CB_GAME_DATA scene_palette_dwords[0x90]; /* GS:0x5251 */
extern volatile cb_u8 render_update_flag_2751; /* GS:0x2751 */
extern volatile cb_u8 CB_FAR *graphics_display_buffer; /* GS:0x5221 */
extern bloodprg_graphics_buffer_ptr CB_GAME_DATA
        graphics_back_buffer; /* GS:0x5229 */
extern cb_u32 palette_low_5251_dwords[]; /* caller ES:0x5251 */
extern cb_u32 palette_low_5851_dwords[]; /* caller ES:0x5851 */
extern volatile cb_u16 graphics_band_top_row; /* GS:0x5239 */
extern volatile cb_u16 graphics_band_bottom_row; /* GS:0x523B */
extern const cb_u8 square_caps_character_map[]; /* GS:0x7362 */
extern const cb_u8 square_caps_advance_table[]; /* GS:0x7412 */
extern const cb_u8 main_font_character_map[]; /* GS:0x7802 */
extern const cb_u8 main_font_advance_table[]; /* GS:0x78B2 */
extern const cb_u8 selected_mask_rows[][32]; /* DS:0x7BB8 */
extern volatile cb_i8 selected_mask_index; /* DS:0x27E3 */

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <string.h>
#else
void CB_FAR *CB_NEAR _fmemcpy(
        void CB_FAR *destination,
        const void CB_FAR *source,
        cb_u16 count);
void CB_FAR *CB_NEAR _fmemset(
        void CB_FAR *destination,
        int value,
        cb_u16 count);
#endif

#if defined(__WATCOMC__)
#pragma intrinsic(_fmemcpy)
#endif

/* Low word is x; high word is y. */
typedef cb_u32 bloodprg_layout_offset_result;

typedef struct bloodprg_gfx_scanline_state {
    cb_u16 row_width;
    cb_u16 row_offset;
    cb_u8 rows_remaining;
    cb_u8 row_count_high;
} bloodprg_gfx_scanline_state;

bloodprg_layout_offset_result CB_FAR layout_offset_calc(cb_u16 columns,
        cb_u16 rows); /* 0x000E62 */
void CB_FAR composite_draw_a(cb_u8 color, cb_u16 x, cb_u16 y,
        cb_u16 width, cb_u16 height); /* 0x0299:0x0BB5 */
void CB_FAR blit_coord_guard_c(cb_u8 color, cb_u16 x, cb_u16 y,
        cb_u16 width, cb_u16 height); /* 0x0299:0x0CDC */
void CB_FAR video_retrace_phase_wait(void); /* 0x0000:0x05D7 */
void CB_NEAR palette_upload_if_dirty(void); /* 0x00178B */
void CB_FAR palette_scene_entries_clear(void); /* 0x00248B */
cb_u16 CB_FAR text_width_dual_font(const cb_u8 CB_NEAR *text,
        int use_main_font); /* 0x0030CD */
void CB_NEAR selected_mask_overlay(void); /* 0x007CB4 */
void CB_NEAR flag_gated_2751(void);       /* 0x00A117 */
int CB_NEAR gfx_scanline_advance(
        bloodprg_gfx_scanline_state *state); /* 0x00AD96 */
void CB_NEAR back_buffer_copy_from(
        cb_u16 x, cb_u16 y, cb_u16 width); /* 0x00933A */

#if defined(__WATCOMC__)
#pragma aux layout_offset_calc parm [ax] [bx] value [bx ax]
/* Watcom reserves BP, so the fifth helper argument remains stack-passed. */
#pragma aux composite_draw_a parm [ax] [bx] [cx] [dx] modify exact []
#pragma aux blit_coord_guard_c parm [ax] [bx] [cx] [dx] modify exact []
#pragma aux back_buffer_copy_from parm [bx] [cx] [dx] modify exact []
#endif

#endif
