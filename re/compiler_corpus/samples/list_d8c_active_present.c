/* Codegen probe for BLOODPRG 0x00A41A. */
#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef unsigned long u32;

#define FAR far
#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA FAR
#endif

typedef volatile u8 FAR *buffer_pointer;

extern volatile u16 list_d8c_active_offset;
extern volatile u16 list_d8c_active_segment;
extern volatile u16 list_d8c_active_layout;
extern volatile u16 list_d8c_active_row_mode;
extern volatile u16 list_d8c_retired_segment;
extern volatile u16 list_d8c_default_entry_segment;
extern volatile u8 resource_frame_presented;
extern volatile u8 resource_draw_via_back_buffer;
extern volatile u8 resource_decode_rectangular;
extern volatile u8 resource_skip_back_buffer_present;
extern volatile u8 resource_unclamped_row_count;
extern volatile u16 resource_vertical_offset;
extern volatile u16 resource_requested_id;
extern buffer_pointer GAME_DATA graphics_display_buffer;
extern buffer_pointer GAME_DATA graphics_back_buffer;

void FAR full_screen_blit_probe(const u32 FAR *source);
void resource_rect_blit_probe(const volatile u8 FAR *source,
        volatile u8 FAR *framebuffer, u16 x, u16 y, u16 width, u16 row_mode);
void resource_payload_decode_rect_probe(const volatile u8 FAR *source,
        volatile u8 FAR *staging, volatile u8 FAR *framebuffer,
        u16 vertical_offset, u16 row_width, u16 rows);

#if defined(__WATCOMC__)
#pragma aux full_screen_blit_probe parm [ds si] modify exact []
#endif

void FAR list_d8c_active_present_probe(void)
{
    const volatile u16 FAR *cursor;
    const volatile u8 FAR *active_resource;
    volatile u8 FAR *display_base;
    volatile u8 FAR *back_buffer_base;
    u16 active_segment;
    u16 width;
    u16 row_mode;
    u16 x;
    u16 y;
    u8 rows;

    active_segment = list_d8c_active_segment;
    list_d8c_active_segment = 0u;
    list_d8c_retired_segment = active_segment;
    if (active_segment == 0u) {
        return;
    }

    active_resource = (const volatile u8 FAR *)MK_FP(
            active_segment, list_d8c_active_offset);
    cursor = (const volatile u16 FAR *)active_resource;
    width = *cursor++ & 0xF9FFu;
    row_mode = *cursor++;
    x = 0u;
    y = 0u;
    if ((list_d8c_active_layout & 0x0400u) == 0u) {
        x = *cursor++;
        y = *cursor++;
    }
    y = (u16)(y + resource_vertical_offset);

    display_base = (volatile u8 FAR *)MK_FP(
            FP_SEG(graphics_display_buffer), 0u);
    back_buffer_base = (volatile u8 FAR *)MK_FP(
            FP_SEG(graphics_back_buffer), 0u);
    resource_frame_presented = 1u;
    rows = (u8)row_mode;

    if ((resource_draw_via_back_buffer & 1u) != 0u) {
        if (rows != 0u) {
            if (rows > 0x82u) {
                row_mode = (u16)((row_mode & 0xFF00u) | 0x82u);
            }
            resource_rect_blit_probe((const volatile u8 FAR *)cursor,
                    back_buffer_base, x, y, width, row_mode);
        }
        full_screen_blit_probe((const u32 FAR *)graphics_back_buffer);
        return;
    }

    if ((resource_skip_back_buffer_present & 1u) == 0u) {
        full_screen_blit_probe((const u32 FAR *)graphics_back_buffer);
    }
    if (resource_decode_rectangular != 0u) {
        resource_payload_decode_rect_probe(active_resource,
                (volatile u8 FAR *)MK_FP(list_d8c_default_entry_segment, 0u),
                display_base, resource_vertical_offset,
                list_d8c_active_layout, list_d8c_active_row_mode);
        return;
    }
    if (rows == 0u) {
        return;
    }
    if ((resource_unclamped_row_count & 1u) == 0u && rows > 0x82u) {
        row_mode = (u16)((row_mode & 0xFF00u) | 0x82u);
    }
    resource_rect_blit_probe((const volatile u8 FAR *)cursor,
            display_base, x, y, width, row_mode);
}
