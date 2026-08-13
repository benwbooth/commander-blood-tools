/* Codegen probe for BLOODPRG 0x00242D. */

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

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

#define PALETTE_COLORS 256u
#define TINT_BANK_SIZE 16u

extern volatile u8 GAME_DATA live_palette_probe[768];

void FAR tint_table_build_banked_probe(
        u16 bank_base,
        volatile u8 GAME_DATA *table);

#if defined(__WATCOMC__)
#pragma aux tint_table_build_banked_probe \
        parm [ax] [bx] modify exact [ax bx]
#endif

void FAR tint_table_build_banked_probe(
        u16 bank_base,
        volatile u8 GAME_DATA *table)
{
    u16 blue;
    u16 green;
    u16 index;
    u16 mapped;
    const volatile u8 GAME_DATA *palette;
    u16 red;
    u16 shade;

    palette = live_palette_probe;
    for (index = 0; index < PALETTE_COLORS; ++index) {
        red = *palette++;
        green = *palette++;
        blue = *palette++;
        shade = (u16)((red * 3u + green * 6u + blue) / 28u);
        if ((i16)shade > 15) {
            shade = 15u;
        }
        mapped = (u16)(bank_base + shade);
        if ((i16)index >= (i16)bank_base
                && (i16)index <= (i16)(bank_base + TINT_BANK_SIZE - 1u)) {
            mapped = index;
        }
        *table++ = (u8)mapped;
    }
}
