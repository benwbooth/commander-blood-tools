#include <dos.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_list.h"
#include "../include/bloodprg_resource.h"

#define LIST_D8C_LAYOUT_MASK 0xF9FFu
#define LIST_D8C_FLAG_NO_COORDINATES 0x0400u
#define LIST_D8C_MAX_ROWS 0x82u

void CB_FAR list_d8c_active_present(void)
{
    const volatile cb_u16 CB_FAR *cursor;
    const volatile cb_u8 CB_FAR *active_resource;
    volatile cb_u8 CB_FAR *display_base;
    volatile cb_u8 CB_FAR *back_buffer_base;
    cb_u16 active_segment;
    cb_u16 width;
    cb_u16 row_mode;
    cb_u16 x;
    cb_u16 y;
    cb_u8 rows;

    active_segment = list_d8c_active_segment;
    list_d8c_active_segment = 0u;
    list_d8c_retired_segment = active_segment;
    if (active_segment == 0u) {
        return;
    }

    active_resource = (const volatile cb_u8 CB_FAR *)MK_FP(
            active_segment, list_d8c_active_offset);
    cursor = (const volatile cb_u16 CB_FAR *)active_resource;
    width = *cursor++ & LIST_D8C_LAYOUT_MASK;
    row_mode = *cursor++;
    x = 0u;
    y = 0u;
    if ((list_d8c_active_layout & LIST_D8C_FLAG_NO_COORDINATES) == 0u) {
        x = *cursor++;
        y = *cursor++;
    }
    y = (cb_u16)(y + resource_vertical_offset_gs);

    display_base = (volatile cb_u8 CB_FAR *)MK_FP(
            FP_SEG(graphics_display_buffer), 0u);
    back_buffer_base = (volatile cb_u8 CB_FAR *)MK_FP(
            FP_SEG(graphics_back_buffer), 0u);
    resource_frame_presented = 1u;

    rows = (cb_u8)row_mode;
    if ((resource_draw_via_back_buffer & 1u) != 0u) {
        if (rows != 0u) {
            if (rows > LIST_D8C_MAX_ROWS) {
                row_mode = (cb_u16)(
                        (row_mode & 0xFF00u) | LIST_D8C_MAX_ROWS);
            }
            resource_rect_blit(
                    (const volatile cb_u8 CB_FAR *)cursor,
                    back_buffer_base,
                    x,
                    y,
                    width,
                    row_mode);
        }
        full_screen_blit((const cb_u32 CB_FAR *)graphics_back_buffer);
        return;
    }

    if ((resource_skip_back_buffer_present & 1u) == 0u) {
        full_screen_blit((const cb_u32 CB_FAR *)graphics_back_buffer);
    }
    if (resource_decode_rectangular != 0u) {
        resource_payload_decode_rect(
                active_resource,
                (volatile cb_u8 CB_FAR *)MK_FP(
                        list_d8c_default_entry_segment, 0u),
                display_base,
                resource_vertical_offset_gs,
                list_d8c_active_layout,
                list_d8c_active_row_mode);
        return;
    }
    if (rows == 0u) {
        return;
    }
    if ((resource_unclamped_row_count & 1u) == 0u &&
            rows > LIST_D8C_MAX_ROWS) {
        row_mode = (cb_u16)((row_mode & 0xFF00u) | LIST_D8C_MAX_ROWS);
    }
    resource_rect_blit(
            (const volatile cb_u8 CB_FAR *)cursor,
            display_base,
            x,
            y,
            width,
            row_mode);
}
