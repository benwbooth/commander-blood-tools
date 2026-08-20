#include "../include/bloodprg_graphics.h"

#define BLOODPRG_PALETTE_COMPONENTS 3u

void CB_FAR palette_range_interpolate(
        const cb_u8 CB_NEAR *source,
        const cb_u8 CB_FAR *target,
        cb_u16 percent,
        cb_u16 first,
        cb_u16 last)
{
    volatile cb_u8 CB_GAME_DATA *destination;
    cb_i8 delta;
    cb_u16 entries;
    cb_u16 offset;
    cb_u8 source_value;
    cb_u8 target_value;

#if defined(__WATCOMC__)
    /* Preserve the caller's target segment across based destination writes. */
    _asm push es;
#endif

    offset = (cb_u16)(first * BLOODPRG_PALETTE_COMPONENTS);
    source += offset;
    target += offset;
    destination = pbm_live_palette + offset;
    entries = (cb_u16)(last - first + 1u);

    do {
        source_value = *source++;
        target_value = *target++;
        delta = (cb_i8)(source_value - target_value);
        *destination++ = (cb_u8)(target_value
                + (cb_i16)delta * (cb_i16)(cb_i8)percent / 100);

        source_value = *source++;
        target_value = *target++;
        delta = (cb_i8)(source_value - target_value);
        *destination++ = (cb_u8)(target_value
                + (cb_i16)delta * (cb_i16)(cb_i8)percent / 100);

        source_value = *source++;
        target_value = *target++;
        delta = (cb_i8)(source_value - target_value);
        *destination++ = (cb_u8)(target_value
                + (cb_i16)delta * (cb_i16)(cb_i8)percent / 100);
    } while (--entries != 0);

#if defined(__WATCOMC__)
    _asm pop es;
#endif
}
