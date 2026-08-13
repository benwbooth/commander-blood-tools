#include <conio.h>

typedef unsigned int u16;
typedef signed int i16;

extern volatile i16 graphics_draw_page_offset;
extern volatile i16 graphics_screen_page_offset;
extern volatile u16 video_crtc_base_port_ds;

void near page_offset_helper(void);
#pragma aux page_offset_helper modify exact [ax dx]

void near page_offset_helper(void)
{
    i16 offset;

    offset = graphics_draw_page_offset;
    if (offset < 0) {
        offset = 0;
    } else {
        offset = (i16)((u16)offset + 0x4000u);
    }
    graphics_draw_page_offset = offset;

    offset = graphics_screen_page_offset;
    if (offset < 0) {
        offset = 0;
    } else {
        offset = (i16)((u16)offset + 0x4000u);
    }
    graphics_screen_page_offset = offset;

    outpw(video_crtc_base_port_ds, ((u16)offset & 0xff00u) | 0x000cu);
}
