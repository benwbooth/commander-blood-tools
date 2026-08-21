#include <dos.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_resource.h"

#define ERROR_OVERLAY_CODING 0u
#define ERROR_OVERLAY_FILE 1u
#define ERROR_OVERLAY_ALLOCATION 2u
#define ERROR_OVERLAY_COLOR 15u
#define ERROR_OVERLAY_ROW_HEIGHT 6u
#define ERROR_OVERLAY_CHARACTER_PITCH 4u
#define VGA_GRAPHICS_SEGMENT 0xa000u

void CB_FAR error_overlay_draw(cb_u16 mode,
        const cb_u8 CB_FAR *detail)
{
    bloodprg_layout_offset_result layout;
    cb_u16 saved_display_segment;
    cb_u16 numeric_x;
    cb_u16 x;
    cb_u16 y;

    saved_display_segment = graphics_display_buffer_segment;
    graphics_display_buffer_segment = VGA_GRAPHICS_SEGMENT;

    if (mode == ERROR_OVERLAY_CODING) {
        layout = layout_offset_calc(
                bloodprg_strlen(error_overlay_coding_text), 1u);
        x = (cb_u16)layout;
        y = (cb_u16)(layout >> 16);
        small_text_render(
                error_overlay_coding_text, x, y, ERROR_OVERLAY_COLOR);
    } else if (mode == ERROR_OVERLAY_FILE) {
        layout = layout_offset_calc(
                bloodprg_strlen(error_overlay_file_text), 2u);
        x = (cb_u16)layout;
        y = (cb_u16)(layout >> 16);
        small_text_render(
                error_overlay_file_text, x, y, ERROR_OVERLAY_COLOR);
        small_text_render(
                detail,
                x,
                (cb_u16)(y + ERROR_OVERLAY_ROW_HEIGHT),
                ERROR_OVERLAY_COLOR);
    } else if (mode == ERROR_OVERLAY_ALLOCATION) {
        layout = layout_offset_calc(
                bloodprg_strlen(error_overlay_allocation_text), 3u);
        x = (cb_u16)layout;
        y = (cb_u16)(layout >> 16);
        small_text_render(
                error_overlay_allocation_text,
                x,
                y,
                ERROR_OVERLAY_COLOR);

        y = (cb_u16)(y + ERROR_OVERLAY_ROW_HEIGHT);
        small_text_render(
                error_overlay_handle_text, x, y, ERROR_OVERLAY_COLOR);
        numeric_x = (cb_u16)(x
                + bloodprg_strlen(error_overlay_handle_text)
                    * ERROR_OVERLAY_CHARACTER_PITCH);
        decimal_append_i16(
                (cb_i16)resource_current_handle_fs,
                error_overlay_number_buffer);
        small_text_render(
                (const cb_u8 CB_NEAR *)error_overlay_number_buffer,
                numeric_x,
                y,
                ERROR_OVERLAY_COLOR);

        y = (cb_u16)(y + ERROR_OVERLAY_ROW_HEIGHT);
        small_text_render(
                error_overlay_free_text, x, y, ERROR_OVERLAY_COLOR);
        decimal_append_i32(
                (cb_i32)resource_free_bytes_gs,
                error_overlay_number_buffer);
        small_text_render(
                (const cb_u8 CB_NEAR *)error_overlay_number_buffer,
                numeric_x,
                y,
                ERROR_OVERLAY_COLOR);
    }

    graphics_display_buffer_segment = saved_display_segment;
}
