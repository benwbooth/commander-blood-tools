#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_input.h"

#if defined(__WATCOMC__)
/* Keep the palette address load after the retrace wait, as in the binary. */
#pragma aux video_retrace_phase_wait modify exact [si]
#endif

void CB_NEAR palette_upload_if_dirty(void)
{
    if ((palette_dirty & 1u) != 0) {
        video_retrace_phase_wait();
        vga_palette_write(live_palette);
        palette_dirty = 0;
        mouse_press_pending = 0;
        mouse_primary_pressed = 0;
    }
}
