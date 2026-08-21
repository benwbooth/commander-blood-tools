#include "../include/bloodprg_graphics.h"

#if defined(__WATCOMC__)
#pragma intrinsic(_fmemset)
#endif

void CB_FAR palette_scene_entries_clear(void)
{
    _fmemset(scene_palette_dwords, 0, sizeof(scene_palette_dwords));
}
