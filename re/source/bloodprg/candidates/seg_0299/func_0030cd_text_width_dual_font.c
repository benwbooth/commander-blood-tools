#include "../include/bloodprg_graphics.h"

cb_u16 CB_FAR text_width_dual_font(
    const cb_u8 CB_FAR *text,
    int use_main_font
)
{
    const cb_u8 CB_NEAR *character_map;
    const cb_u8 CB_NEAR *advance_table;
    cb_u16 width;
    cb_u8 character;

    if (use_main_font == 0) {
        character_map = square_caps_character_map;
        advance_table = square_caps_advance_table;
    } else {
        character_map = main_font_character_map;
        advance_table = main_font_advance_table;
    }

    width = 0;
    while ((character = *text++) != 0) {
        width = (cb_u16)(width + advance_table[character_map[character]]);
    }

    return (cb_u16)(width - 2u);
}
