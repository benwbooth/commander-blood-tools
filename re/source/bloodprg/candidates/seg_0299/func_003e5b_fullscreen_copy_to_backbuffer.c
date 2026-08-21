#include "../include/bloodprg_graphics.h"

void CB_SAVE_REGS CB_FAR fullscreen_copy_to_backbuffer(
        const cb_u32 CB_NEAR *source)
{
    _fmemcpy((void CB_FAR *)graphics_back_buffer,
            (const void CB_FAR *)source, 0xfa00u);
}
