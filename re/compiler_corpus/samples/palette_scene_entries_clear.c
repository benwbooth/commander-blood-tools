typedef unsigned long u32;

#include <string.h>

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA FAR
#endif

#define SCENE_PALETTE_DWORDS 0x90u

extern u32 GAME_DATA scene_palette_dwords[SCENE_PALETTE_DWORDS];

void FAR palette_scene_entries_clear_probe(void);

#if defined(__WATCOMC__)
#pragma intrinsic(_fmemset)
#pragma aux palette_scene_entries_clear_probe modify exact []
#endif

void FAR palette_scene_entries_clear_probe(void)
{
#if defined(__WATCOMC__)
    _asm push eax;
    _asm push es;
#endif

    _fmemset(scene_palette_dwords, 0, sizeof(scene_palette_dwords));

#if defined(__WATCOMC__)
    _asm pop es;
    _asm pop eax;
#endif
}
