#include "../include/bloodprg_graphics.h"

#define BLOODPRG_PALETTE_COLORS 256u
#define BLOODPRG_PALETTE_COMPONENTS 3u
#define BLOODPRG_TINT_BANK_SIZE 16u

void CB_FAR tint_table_build_banked(
        cb_u16 bank_base,
        volatile cb_u8 CB_GAME_DATA *table)
{
    cb_u16 blue;
    cb_u16 green;
    cb_u16 index;
    cb_u16 mapped;
    const volatile cb_u8 CB_GAME_DATA *palette;
    cb_u16 red;
    cb_u16 shade;

    palette = pbm_live_palette;
    for (index = 0; index < BLOODPRG_PALETTE_COLORS; ++index) {
        red = *palette++;
        green = *palette++;
        blue = *palette++;
        shade = (cb_u16)((red * 3u + green * 6u + blue) / 28u);
        if ((cb_i16)shade > 15) {
            shade = 15u;
        }
        mapped = (cb_u16)(bank_base + shade);
        if ((cb_i16)index >= (cb_i16)bank_base
                && (cb_i16)index
                        <= (cb_i16)(bank_base + BLOODPRG_TINT_BANK_SIZE - 1u)) {
            mapped = index;
        }
        *table++ = (cb_u8)mapped;
    }
}
