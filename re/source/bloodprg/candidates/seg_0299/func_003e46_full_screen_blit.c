#include "../include/bloodprg_graphics.h"

void CB_SAVE_REGS CB_FAR full_screen_blit(const cb_u32 CB_FAR *source)
{
    _fmemcpy((void CB_FAR *)graphics_display_buffer,
            (const void CB_FAR *)source, 0xfa00u);
}
