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
extern bloodprg_graphics_buffer_ptr CB_GAME_DATA
        graphics_display_buffer; /* GS:0x5221 */
extern bloodprg_graphics_buffer_ptr CB_GAME_DATA
        graphics_back_buffer; /* GS:0x5229 */
extern volatile cb_u8 CB_GAME_DATA
        pbm_ship_palette_limit; /* GS:0x24F3 */
extern volatile cb_u8 CB_GAME_DATA
        pbm_scene_palette_limit; /* GS:0x274F */
extern volatile cb_u8 CB_GAME_DATA
        pbm_palette_refresh; /* GS:0x5B53 */
extern volatile cb_u8 CB_GAME_DATA
        pbm_palette_dirty; /* GS:0x5B55 */
extern volatile cb_u8 CB_GAME_DATA
        pbm_transparent_zero; /* GS:0x5B57 */
extern volatile cb_u8 CB_GAME_DATA
        pbm_live_palette[768]; /* GS:0x5251 */
extern bloodprg_graphics_buffer_ptr CB_GAME_DATA
        graphics_draw_framebuffer; /* GS:0x5219 */
extern bloodprg_graphics_buffer_ptr CB_GAME_DATA
        graphics_screen_buffer; /* GS:0x521D */
extern cb_u32 palette_low_5251_dwords[]; /* caller ES:0x5251 */
extern cb_u32 palette_low_5851_dwords[]; /* caller ES:0x5851 */
extern volatile cb_u16 CB_GAME_DATA graphics_band_top_row; /* GS:0x5239 */
extern volatile cb_u16 CB_GAME_DATA graphics_band_bottom_row; /* GS:0x523B */
extern volatile cb_i16 CB_GAME_DATA graphics_clip_left; /* GS:0x5235 */
extern volatile cb_i16 CB_GAME_DATA graphics_clip_right; /* GS:0x5237 */
extern volatile cb_u8 CB_GAME_DATA
        graphics_span_remap_enabled; /* GS:0x5B56 */
extern const cb_u8 CB_GAME_DATA
        graphics_span_remap_table[256]; /* GS:0x5F11 */
extern const cb_u8 square_caps_character_map[]; /* GS:0x7362 */
extern const cb_u8 square_caps_advance_table[]; /* GS:0x7412 */
extern const cb_u8 CB_GAME_DATA
        square_caps_draw_character_map[256]; /* GS:0x7362 */
extern const cb_u8 CB_GAME_DATA
        square_caps_draw_advance_table[]; /* GS:0x7412 */
extern const cb_u8 CB_GAME_DATA
        square_caps_draw_glyphs[]; /* GS:0x7442 */
extern volatile cb_u16 CB_GAME_DATA
        square_caps_draw_width; /* GS:0x27CD */
extern const cb_u8 main_font_character_map[]; /* GS:0x7802 */
extern const cb_u8 main_font_advance_table[]; /* GS:0x78B2 */
extern const cb_u8 CB_GAME_DATA
        main_font_draw_character_map[256]; /* GS:0x7802 */
extern const cb_u8 CB_GAME_DATA
        main_font_draw_advance_table[]; /* GS:0x78B2 */
extern const cb_u8 CB_GAME_DATA
        main_font_draw_glyphs[]; /* SS:0x7908 */
extern volatile cb_u16 CB_GAME_DATA
        main_font_draw_width; /* GS:0x27CD */
extern const cb_u8 CB_GAME_DATA
        subtitle_console_character_map[256]; /* GS:0x70FA */
extern const cb_u8 CB_GAME_DATA
        subtitle_console_glyphs[]; /* SS:0x71AA */
extern volatile cb_u16 CB_GAME_DATA
        subtitle_reveal_cursor; /* GS:0x5E58 */
extern const cb_u8 CB_GAME_DATA
        small_font_character_map[256]; /* GS:0x6FA8 */
extern const cb_u8 CB_GAME_DATA
        small_font_glyphs[]; /* SS:0x7028 */
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
void CB_FAR gfx_horizontal_span(cb_u8 color, cb_u16 x, cb_u16 y,
        cb_u16 width); /* 0x0299:0x031C */
void CB_FAR gfx_vertical_span(cb_u8 color, cb_u16 x, cb_u16 y,
        cb_u16 height); /* 0x0299:0x0391 */
void CB_FAR framebuffer_rect_palette_remap(
        const cb_u8 CB_FAR *remap_table,
        cb_u16 x,
        cb_u16 y,
        cb_u16 width,
        cb_u16 height); /* 0x0299:0x040E */
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
void CB_NEAR resource_palette_file_blocks_apply(cb_u16 file_handle,
        volatile cb_u16 *header_buffer,
        cb_u32 *remaining_bytes); /* 0x004086 */
int CB_NEAR gfx_scanline_advance(
        bloodprg_gfx_scanline_state *state); /* 0x00AD96 */
void CB_NEAR back_buffer_copy_from(
        cb_u16 x, cb_u16 y, cb_u16 width); /* 0x00933A */
void CB_FAR blit_fill_row_5221(cb_u8 color); /* 0x003D7B */
void CB_FAR back_buffer_fill(cb_u8 color);   /* 0x003DBF */
void CB_FAR full_screen_blit(
        const cb_u32 CB_NEAR *source); /* 0x003E46 */
void CB_FAR fullscreen_copy_to_backbuffer(
        const cb_u32 CB_NEAR *source); /* 0x003E5B */
void CB_FAR bridge_panorama_frame_unpack(
        const cb_u8 CB_FAR *source); /* 0x002D50 */
const cb_u8 CB_FAR *CB_FAR font8x8_text_draw_display(
        const cb_u8 CB_FAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u16 color_and_limit); /* 0x003066: DH=limit, DL=color */
void CB_FAR square_caps_text_draw_display(
        const cb_u8 CB_FAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u8 color); /* 0x003106 */
void CB_FAR planar_ui_text_render_10row(
        const cb_u8 CB_FAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u8 color); /* 0x003428 */
void CB_FAR planar_dialogue_text_render(
        const cb_u8 CB_FAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u8 color); /* 0x00356E */
void CB_FAR subtitle_reveal_draw_wrapper(
        const cb_u8 CB_NEAR *line,
        cb_u16 x,
        cb_u16 y); /* 0x003630 */
void CB_FAR small_text_render(
        const cb_u8 CB_NEAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u8 color); /* 0x0036EA */
void CB_FAR main_font_text_draw_display(
        const cb_u8 CB_FAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u8 color); /* 0x003192 */
void CB_FAR subtitle_reveal_pump(void); /* 0x0093F5 */

#if defined(__WATCOMC__)
#pragma aux layout_offset_calc parm [ax] [bx] value [bx ax]
#pragma aux gfx_horizontal_span parm [ax] [bx] [cx] [dx] modify exact []
#pragma aux gfx_vertical_span parm [ax] [bx] [cx] [dx] modify exact [bx]
#pragma aux framebuffer_rect_palette_remap \
        parm caller [ds si] [bx] [cx] [dx] modify exact []
/* The remapper recovers entry BP; the other fifth arguments remain stack-passed. */
#pragma aux composite_draw_a parm [ax] [bx] [cx] [dx] modify exact []
#pragma aux blit_coord_guard_c parm [ax] [bx] [cx] [dx] modify exact []
#pragma aux back_buffer_copy_from parm [bx] [cx] [dx] modify exact []
#pragma aux blit_fill_row_5221 parm [ax] modify exact []
#pragma aux back_buffer_fill parm [ax] modify exact []
#pragma aux full_screen_blit parm [si] modify exact []
#pragma aux fullscreen_copy_to_backbuffer parm [si] modify exact []
#pragma aux bridge_panorama_frame_unpack parm [ds si]
#pragma aux font8x8_text_draw_display \
        parm [ds si] [ax] [bx] [dx] value [ds si] modify exact [si]
#pragma aux square_caps_text_draw_display \
        parm [ds si] [bx] [dx] [ax] modify exact []
#pragma aux planar_ui_text_render_10row \
        parm [ds si] [bx] [dx] [ax] modify exact []
#pragma aux planar_dialogue_text_render \
        parm [ds si] [bx] [dx] [ax] modify exact []
#pragma aux subtitle_reveal_draw_wrapper \
        parm [si] [bx] [dx] modify exact []
#pragma aux small_text_render \
        parm [si] [ax] [bx] [dx] modify exact []
#pragma aux main_font_text_draw_display \
        parm [ds si] [bx] [dx] [ax] modify exact []
#endif

#endif
