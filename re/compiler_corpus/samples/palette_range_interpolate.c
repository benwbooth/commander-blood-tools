/* Codegen probe for BLOODPRG 0x0023C5. */

typedef unsigned char u8;
typedef signed char i8;
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

extern volatile u8 GAME_DATA live_palette_probe[768];

void FAR palette_range_interpolate_probe(
        const u8 FAR *source,
        const u8 FAR *target,
        i8 percent,
        u16 first,
        u16 last)
{
    volatile u8 GAME_DATA *destination;
    i8 delta;
    u16 entries;
    u16 offset;
    u8 source_value;
    u8 target_value;

#if defined(__WATCOMC__)
    _asm push es;
#endif

    offset = (u16)(first * 3u);
    source += offset;
    target += offset;
    destination = live_palette_probe + offset;
    entries = (u16)(last - first + 1u);

    do {
        source_value = *source++;
        target_value = *target++;
        delta = (i8)(source_value - target_value);
        *destination++ = (u8)(target_value
                + (i16)delta * (i16)percent / 100);

        source_value = *source++;
        target_value = *target++;
        delta = (i8)(source_value - target_value);
        *destination++ = (u8)(target_value
                + (i16)delta * (i16)percent / 100);

        source_value = *source++;
        target_value = *target++;
        delta = (i8)(source_value - target_value);
        *destination++ = (u8)(target_value
                + (i16)delta * (i16)percent / 100);
    } while (--entries != 0);

#if defined(__WATCOMC__)
    _asm pop es;
#endif
}
