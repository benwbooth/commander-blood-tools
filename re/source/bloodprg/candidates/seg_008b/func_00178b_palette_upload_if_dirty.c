#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_input.h"

#if defined(__WATCOMC__)
/* Keep the palette address load after the retrace wait, as in the binary. */
#pragma aux video_retrace_phase_wait modify exact [si]
/* Suppress unrelated saves; AX is explicitly preserved and SI is the result. */
#pragma aux palette_upload_if_dirty modify exact [ax bx cx dx si di es]
#endif

void CB_NEAR palette_upload_if_dirty(void)
{
#if defined(__WATCOMC__)
    /* The recovered callers require AX to survive this no-result helper. */
    _asm push ax;
#endif

    if ((palette_dirty & 1u) != 0) {
        video_retrace_phase_wait();
        vga_palette_write(live_palette);
        palette_dirty = 0;
        mouse_press_pending = 0;
        mouse_primary_pressed = 0;
    }

#if defined(__WATCOMC__)
    _asm pop ax;
#endif
}
