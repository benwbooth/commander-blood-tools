#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_input.h"

void CB_NEAR palette_upload_if_dirty(void)
{
    if ((palette_dirty & 1u) == 0) {
        return;
    }

    video_retrace_wait();
    vga_palette_write(live_palette);
    palette_dirty = 0;
    mouse_press_pending = 0;
    mouse_primary_pressed = 0;
}
