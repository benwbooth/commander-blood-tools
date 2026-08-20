#ifndef BLOODPRG_GRAPHICS_H
#define BLOODPRG_GRAPHICS_H

#include "bloodprg_common.h"
#include "bloodprg_hardware.h"
#include "bloodprg_input.h"

typedef volatile cb_u8 CB_FAR *bloodprg_graphics_buffer_ptr;

typedef struct bloodprg_centered_text_line {
    cb_u16 character_count;
    cb_u16 centered_x;
} bloodprg_centered_text_line;

typedef struct bloodprg_subtitle_frame_primitive {
    cb_i16 kind;
    cb_u16 x;
    cb_u16 y;
    cb_u16 extent;
} bloodprg_subtitle_frame_primitive;

typedef struct bloodprg_viewport_descriptor {
    cb_u16 field_00;
    cb_u16 field_02;
    cb_u32 field_04;
    cb_u16 width;
    cb_u16 height;
    cb_u32 field_0c;
} bloodprg_viewport_descriptor;

typedef char bloodprg_viewport_descriptor_size_must_be_16[
        sizeof(bloodprg_viewport_descriptor) == 16 ? 1 : -1];

/* SS:0x0AF2 in 0x007CE8; ordinary data shares the runtime stack segment. */
extern volatile bloodprg_centered_text_line CB_NEAR
        centered_text_line_layout[];

extern bloodprg_graphics_buffer_ptr CB_GAME_DATA
        graphics_work_surface; /* GS:0x0ABC */
extern volatile cb_u8 palette_dirty; /* game data:0x5B55 */
extern volatile cb_u8 page_flip_transparent_zero; /* DS:0x5B57 */
extern volatile cb_u8 live_palette[768]; /* game data:0x5251 */
/* First 192 RGB entries of live_palette, addressed through GS by 0x00248B. */
extern cb_u32 CB_GAME_DATA scene_palette_dwords[0x90]; /* GS:0x5251 */
extern volatile cb_u8 render_update_flag_2751; /* GS:0x2751 */
extern volatile cb_u8 CB_GAME_DATA
        render_update_flag_2751_gs; /* explicit GS:0x2751 alias */
extern bloodprg_graphics_buffer_ptr CB_GAME_DATA
        graphics_display_buffer; /* GS:0x5221 */
extern bloodprg_graphics_buffer_ptr
        graphics_display_buffer_ds; /* DS:0x5221 alias */
extern volatile cb_u16 CB_GAME_DATA
        graphics_display_buffer_segment; /* GS:0x5223 */
extern bloodprg_graphics_buffer_ptr
        bridge_panorama_load_buffer; /* DS:0x5221 alias */
extern bloodprg_graphics_buffer_ptr CB_GAME_DATA
        graphics_back_buffer; /* GS:0x5229 */
extern bloodprg_graphics_buffer_ptr
        graphics_back_buffer_ds; /* DS:0x5229 alias */
extern volatile bloodprg_viewport_descriptor CB_FAR *
        graphics_viewport_descriptor; /* DS:0x522D */
extern volatile cb_u8 CB_GAME_DATA
        pbm_ship_palette_limit; /* GS:0x24F3 */
extern volatile cb_u8 CB_GAME_DATA
        pbm_scene_palette_limit; /* GS:0x274F */
extern volatile cb_u8 CB_GAME_DATA
        pbm_palette_refresh; /* GS:0x5B53 */
extern volatile cb_u8 pbm_palette_refresh_ds; /* DS:0x5B53 alias */
extern volatile cb_u8 CB_GAME_DATA
        pbm_palette_dirty; /* GS:0x5B55 */
extern volatile cb_u8 CB_GAME_DATA
        pbm_transparent_zero; /* GS:0x5B57 */
extern volatile cb_u8 pbm_transparent_zero_ds; /* DS:0x5B57 alias */
extern volatile char CB_GAME_DATA
        back_buffer_init_image_path[]; /* DS:0x00EA */
extern volatile char CB_GAME_DATA
        backbuffer_clear_image_path[]; /* DS:0x00E3 */
extern volatile char CB_GAME_DATA
        scene_transition_image_path[]; /* DS:0x00F3 */
extern volatile char scene_transition_image_path_ds[]; /* DS:0x00F3 alias */
extern volatile cb_u8 CB_GAME_DATA
        pbm_live_palette[768]; /* GS:0x5251 */
extern volatile cb_u8 CB_GAME_DATA
        bridge_panorama_palette[768]; /* GS:0x5B58 */
extern bloodprg_graphics_buffer_ptr CB_GAME_DATA
        graphics_draw_framebuffer; /* GS:0x5219 */
extern bloodprg_graphics_buffer_ptr
        graphics_draw_framebuffer_ds; /* DS:0x5219 alias */
extern bloodprg_graphics_buffer_ptr CB_GAME_DATA
        graphics_screen_buffer; /* GS:0x521D */
extern volatile cb_i16
        graphics_draw_page_offset; /* DS:0x5219 offset-word alias */
extern volatile cb_i16
        graphics_screen_page_offset; /* DS:0x521D offset-word alias */
extern bloodprg_graphics_buffer_ptr
        graphics_screen_buffer_ds; /* DS:0x521D far-pointer alias */
extern volatile cb_u8
        main_loop_hud_refresh_enabled; /* DS:0x0ADF */
extern const cb_u8
        main_loop_hud_text[]; /* DS:0x0166 */
extern const cb_u8 CB_GAME_DATA
        error_overlay_coding_text[]; /* DS:0x002E */
extern const cb_u8 CB_GAME_DATA
        error_overlay_file_text[]; /* DS:0x0041 */
extern const cb_u8 CB_GAME_DATA
        error_overlay_allocation_text[]; /* DS:0x0055 */
extern const cb_u8 CB_GAME_DATA
        error_overlay_handle_text[]; /* DS:0x0073 */
extern const cb_u8 CB_GAME_DATA
        error_overlay_free_text[]; /* DS:0x007D */
extern char CB_GAME_DATA
        error_overlay_number_buffer[]; /* GS:0x0AF2 */
extern cb_u32 palette_low_5251_dwords[]; /* caller ES:0x5251 */
extern cb_u32 palette_low_5851_dwords[]; /* caller ES:0x5851 */
extern cb_u8
        palette_transition_source[768]; /* DS:0x5851 */
extern cb_u8 CB_GAME_DATA
        palette_transition_source_gs[768]; /* GS:0x5851 */
extern cb_u8 CB_GAME_DATA
        palette_transition_target[768]; /* GS:0x5551 */
extern volatile cb_u16
        palette_transition_increment; /* DS:0x524D */
extern volatile cb_u16
        palette_transition_percent; /* DS:0x524F */
extern volatile cb_u8
        palette_transition_first; /* DS:0x5B51 */
extern volatile cb_u8
        palette_transition_last; /* DS:0x5B52 */
extern const cb_u8 CB_NEAR *
        framebuffer_transition_remap_table; /* DS:0x0AC8 */
extern volatile cb_u8
        framebuffer_transition_total_steps; /* DS:0x0ADA */
extern volatile cb_u8
        framebuffer_transition_current_step; /* DS:0x0ADB */
extern volatile cb_u16 CB_GAME_DATA graphics_band_top_row; /* GS:0x5239 */
extern volatile cb_u16 CB_GAME_DATA graphics_band_bottom_row; /* GS:0x523B */
extern volatile cb_i16 CB_GAME_DATA graphics_clip_left; /* GS:0x5235 */
extern volatile cb_i16 CB_GAME_DATA graphics_clip_right; /* GS:0x5237 */
extern volatile cb_u8 CB_GAME_DATA
        graphics_span_remap_enabled; /* GS:0x5B56 */
extern volatile cb_u8 CB_GAME_DATA
        graphics_span_remap_table[256]; /* GS:0x5F11 */
extern cb_u32 presentation_palette_dwords[0x30]; /* caller ES:0x59D1 */
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
extern volatile cb_u16 subtitle_reveal_delay; /* DS:0x0B31 */
extern volatile cb_u16 subtitle_opening_frame_pulse; /* DS:0x0B37 */
extern volatile cb_u16 subtitle_text_speed_step; /* DS:0x0ACA */
extern volatile cb_u16 presentation_text_origin_x; /* DS:0x5E5C */
extern volatile cb_u16 presentation_text_origin_y; /* DS:0x5E5E */
/* Original BP addressing selects SS; shipped execution has SS == DS. */
extern const bloodprg_subtitle_frame_primitive
        subtitle_frame_primitives_primary[]; /* SS:0x5E6F */
extern const bloodprg_subtitle_frame_primitive
        subtitle_frame_primitives_secondary[]; /* SS:0x5EAF */
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
#if defined(__WATCOMC__)
void CB_FAR framebuffer_rect_palette_remap_ds_bp(
        const cb_u8 CB_NEAR *remap_table,
        cb_u16 x,
        cb_u16 y,
        cb_u16 width,
        cb_u16 height);
#endif
void CB_FAR framebuffer_rect_interpolate_and_remap_step(
        const bloodprg_rect_i16 CB_NEAR *source,
        const bloodprg_rect_i16 CB_NEAR *target); /* 0x001E5D */
void CB_FAR gfx_clipped_span_fill(cb_u8 color, cb_u16 x, cb_u16 y,
        cb_u16 width); /* 0x0299:0x0A2B */
void CB_FAR gfx_clipped_planar_vertical_span(cb_u8 color, cb_u16 x,
        cb_u16 y, cb_u16 height); /* 0x0299:0x0B23 */
void CB_FAR framebuffer_noise_rect(cb_u16 mode, cb_u16 x, cb_u16 y,
        cb_u16 width, cb_u16 height); /* 0x0299:0x0BF5 */
void CB_FAR composite_draw_a(cb_u8 color, cb_u16 x, cb_u16 y,
        cb_u16 width, cb_u16 height); /* 0x0299:0x0BB5 */
void CB_FAR framebuffer_rect_fill(cb_u8 color, cb_u16 x, cb_u16 y,
        cb_u16 width, cb_u16 height); /* 0x0299:0x0CDC */
void CB_FAR vga_planar_to_chunky(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *destination); /* 0x0299:0x0EE0 */
void CB_FAR chunky_to_planar_framebuffer(
        const volatile cb_u8 CB_FAR *source); /* 0x0299:0x0F3E */
cb_i16 CB_FAR back_buffer_init(void); /* 0x008B:0x0929 */
cb_i16 CB_FAR backbuffer_clear_flags(void); /* 0x008B:0x0967 */
void CB_NEAR page_offset_helper(void); /* 0x0017AF */
void CB_NEAR main_loop_hud_refresh(void); /* 0x001A93 */
void CB_NEAR scene_transition_step(cb_u16 link_target_offset); /* 0x001855 */
void CB_FAR error_overlay_draw(cb_u16 mode,
        const cb_u8 CB_FAR *detail); /* 0x000D75 */
void CB_FAR video_retrace_phase_wait(void); /* 0x0000:0x05D7 */
void CB_NEAR palette_upload_if_dirty(void); /* 0x00178B */
cb_i16 CB_FAR palette_blend_remap_table_build(
        cb_i16 negative_percent,
        cb_u16 target_red,
        cb_u16 target_green,
        cb_u16 target_blue,
        volatile cb_u8 CB_GAME_DATA *table); /* 0x0022E0 */
void CB_FAR palette_range_interpolate(
        const cb_u8 CB_FAR *source,
        const cb_u8 CB_FAR *target,
        cb_i8 percent,
        cb_u16 first,
        cb_u16 last); /* 0x0023C5 */
#if defined(__WATCOMC__)
void CB_FAR palette_range_interpolate_ds(
        const cb_u8 CB_NEAR *source,
        const cb_u8 CB_FAR *target,
        cb_u16 percent,
        cb_u16 first,
        cb_u16 last);
#else
#define palette_range_interpolate_ds(source, target, percent, first, last) \
        palette_range_interpolate( \
            (source), (target), (cb_i8)(percent), (first), (last))
#endif
void CB_FAR palette_transition_step(void); /* 0x001F78 */
void CB_FAR tint_table_build_banked(
        cb_u16 bank_base,
        volatile cb_u8 CB_GAME_DATA *table); /* 0x00242D */
void CB_FAR palette_scene_entries_clear(void); /* 0x00248B */
cb_u16 CB_FAR text_width_dual_font(const cb_u8 CB_NEAR *text,
        int use_main_font); /* 0x0030CD */
#if defined(__WATCOMC__)
cb_u16 CB_FAR text_width_dual_font_far(
        const cb_u8 CB_FAR *text,
        int use_main_font);
#else
#define text_width_dual_font_far(text, use_main_font) \
    text_width_dual_font((const cb_u8 CB_NEAR *)(text), (use_main_font))
#endif
void CB_NEAR selected_mask_overlay(void); /* 0x007CB4 */
void CB_NEAR flag_gated_2751(void);       /* 0x00A117 */
void CB_NEAR resource_palette_file_blocks_apply(cb_u16 file_handle,
        volatile cb_u16 *header_buffer,
        cb_u32 *remaining_bytes); /* 0x004086 */
int CB_NEAR gfx_scanline_advance(
        bloodprg_gfx_scanline_state *state); /* 0x00AD96 */
void CB_NEAR back_buffer_copy_from(
        cb_u16 x, cb_u16 y, cb_u16 width); /* 0x00933A */
void CB_SAVE_REGS CB_FAR blit_fill_row_5221(cb_u8 color); /* 0x003D7B */
void CB_SAVE_REGS CB_FAR back_buffer_fill(cb_u8 color);   /* 0x003DBF */
void CB_FAR full_screen_blit(
        const cb_u32 CB_FAR *source); /* 0x003E46 */
void CB_FAR fullscreen_copy_to_backbuffer(
        const cb_u32 CB_NEAR *source); /* 0x003E5B */
#if defined(__WATCOMC__)
void CB_FAR fullscreen_copy_to_backbuffer_far(
        const cb_u32 CB_FAR *source);
#else
#define fullscreen_copy_to_backbuffer_far(source) \
        fullscreen_copy_to_backbuffer((const cb_u32 CB_NEAR *)(source))
#endif
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
#if defined(__WATCOMC__)
void CB_FAR planar_ui_text_render_10row_ds(
        const cb_u8 CB_NEAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u8 color);
#else
#define planar_ui_text_render_10row_ds planar_ui_text_render_10row
#endif
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
#if defined(__WATCOMC__)
void CB_FAR small_text_render_far(
        const cb_u8 CB_FAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u8 color);
#endif
void CB_FAR main_font_text_draw_display(
        const cb_u8 CB_FAR *text,
        cb_u16 x,
        cb_u16 y,
        cb_u8 color); /* 0x003192 */
void CB_FAR subtitle_reveal_pump(void); /* 0x0093F5 */
void CB_NEAR list_walk_f18(void); /* 0x007CE8 */

#if defined(__WATCOMC__)
#pragma aux layout_offset_calc parm [ax] [bx] value [bx ax]
#pragma aux gfx_horizontal_span parm [ax] [bx] [cx] [dx] modify exact []
#pragma aux gfx_vertical_span parm [ax] [bx] [cx] [dx] modify exact [bx]
/* Watcom reserves BP, so evaluate all five C arguments before installing the
 * height register around the real far call. */
#pragma aux framebuffer_rect_palette_remap_ds_bp = \
        "push bp" \
        "mov bp,ax" \
        "call far ptr framebuffer_rect_palette_remap" \
        "pop bp" \
        parm [si] [bx] [cx] [dx] [ax] modify exact []
#pragma aux framebuffer_rect_interpolate_and_remap_step \
        parm [si] [di] modify exact []
#pragma aux gfx_clipped_span_fill parm [ax] [bx] [cx] [dx] modify exact []
#pragma aux gfx_clipped_planar_vertical_span \
        parm [ax] [bx] [cx] [dx] modify exact []
#pragma aux framebuffer_noise_rect parm caller [ax] [bx] [cx] [dx] modify exact []
/* These three routines recover entry BP; other fifth arguments remain stack-passed. */
#pragma aux composite_draw_a parm [ax] [bx] [cx] [dx] modify exact []
#pragma aux framebuffer_rect_fill parm caller [ax] [bx] [cx] [dx] modify exact []
#pragma aux page_offset_helper modify exact [ax dx]
#pragma aux main_loop_hud_refresh modify exact [ax bx cx dx di]
#pragma aux video_retrace_phase_wait modify exact []
#pragma aux palette_blend_remap_table_build \
        parm [ax] [bx] [cx] [dx] [di] value [ax] modify exact []
#pragma aux palette_range_interpolate_ds "palette_range_interpolate_" \
        parm [si] [es di] [ax] [bx] [dx] \
        modify exact []
#pragma aux palette_transition_step modify exact []
#pragma aux tint_table_build_banked \
        parm [ax] [bx] modify exact [ax bx]
#pragma aux back_buffer_copy_from parm [bx] [cx] [dx] modify exact []
#pragma aux blit_fill_row_5221 parm [ax] modify exact []
#pragma aux back_buffer_fill parm [ax] modify exact []
#pragma aux fullscreen_copy_to_backbuffer parm [si] modify exact []
#pragma aux planar_ui_text_render_10row_ds "planar_ui_text_render_10row_" \
        parm [si] [bx] [dx] [ax] modify exact []
#pragma aux subtitle_reveal_draw_wrapper \
        parm [si] [bx] [dx] modify exact []
#pragma aux subtitle_reveal_pump modify exact [bx cx dx di es]
#pragma aux small_text_render \
        parm [si] [ax] [bx] [dx] modify exact []
#endif

#endif
