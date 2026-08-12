#include "../include/bloodprg_graphics.h"

void CB_FAR fullscreen_copy_to_backbuffer(const cb_u32 CB_NEAR *source)
{
#if defined(__WATCOMC__)
    _asm push ax;
    _asm push es;
#endif

    _fmemcpy((void CB_FAR *)graphics_back_buffer,
            (const void CB_FAR *)source, 0xfa00u);

#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop ax;
#endif
}
