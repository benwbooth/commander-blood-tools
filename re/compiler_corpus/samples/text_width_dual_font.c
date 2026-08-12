/*
 * Codegen probe for BLOODPRG 0x0030CD.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern const u8 square_caps_character_map[];
extern const u8 square_caps_advance_table[];
extern const u8 main_font_character_map[];
extern const u8 main_font_advance_table[];

u16 FAR text_width_dual_font_probe(const u8 NEAR *text, int use_main_font)
{
    const u8 NEAR *character_map;
    const u8 NEAR *advance_table;
    u16 width;
    u8 character;

    if (use_main_font == 0) {
        character_map = square_caps_character_map;
        advance_table = square_caps_advance_table;
    } else {
        character_map = main_font_character_map;
        advance_table = main_font_advance_table;
    }

    width = 0;
    while ((character = *text++) != 0) {
        width = (u16)(width + advance_table[character_map[character]]);
    }

    return (u16)(width - 2u);
}
