#include "../include/bloodprg_graphics.h"

#if defined(__WATCOMC__)
#pragma intrinsic(_fmemset)
#pragma aux palette_scene_entries_clear modify exact []
#endif

void CB_FAR palette_scene_entries_clear(void)
{
#if defined(__WATCOMC__)
    /* The intrinsic otherwise leaks two registers preserved by the binary. */
    _asm push eax;
    _asm push es;
#endif

    _fmemset(scene_palette_dwords, 0, sizeof(scene_palette_dwords));

#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop eax;
#endif
}
