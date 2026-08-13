/* Codegen probe for BLOODPRG 0x0022E0. */

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
#define PALETTE_BLEND_LIMIT 0x0bb8u

extern volatile u8 GAME_DATA live_palette_probe[768];

i16 FAR palette_blend_remap_table_build_probe(
        i16 negative_percent,
        u16 target_red,
        u16 target_green,
        u16 target_blue,
        volatile u8 GAME_DATA *table);

#if defined(__WATCOMC__)
#pragma aux palette_blend_remap_table_build_probe \
        parm [ax] [bx] [cx] [dx] [di] value [ax] modify exact []
#endif

i16 FAR palette_blend_remap_table_build_probe(
        i16 negative_percent,
        u16 target_red,
        u16 target_green,
        u16 target_blue,
        volatile u8 GAME_DATA *table)
{
    u16 best_distance;
    i16 best_index;
    u16 blended_blue;
    u16 blended_green;
    u16 blended_red;
    u16 candidate;
    const volatile u8 GAME_DATA *candidate_color;
    u16 delta;
    u16 distance;
    u16 percent;
    u16 source;
    const volatile u8 GAME_DATA *source_color;
    u16 source_weight;
    u16 target_blue_scaled;
    u16 target_green_scaled;
    u16 target_red_scaled;

#if defined(__WATCOMC__)
    _asm push es;
#endif

    percent = (u16)(0u - (u16)negative_percent);
    target_red_scaled = (u16)(percent * target_red / 100u);
    target_green_scaled = (u16)(percent * target_green / 100u);
    target_blue_scaled = (u16)(percent * target_blue / 100u);
    source_weight = (u16)(100u - percent);
    source_color = live_palette_probe;

    for (source = 0; source < PALETTE_COLORS; ++source) {
        blended_red = (u16)(
                (u16)*source_color++ * source_weight / 100u
                + target_red_scaled);
        blended_green = (u16)(
                (u16)*source_color++ * source_weight / 100u
                + target_green_scaled);
        blended_blue = (u16)(
                (u16)*source_color++ * source_weight / 100u
                + target_blue_scaled);

        best_index = -1;
        best_distance = PALETTE_BLEND_LIMIT;
        candidate_color = live_palette_probe;
        for (candidate = 0; candidate < PALETTE_COLORS; ++candidate) {
            delta = (u16)(blended_red - *candidate_color++);
            if ((i16)delta < 0) {
                delta = (u16)(0u - delta);
            }
            distance = (u16)(delta * delta);

            delta = (u16)(blended_green - *candidate_color++);
            if ((i16)delta < 0) {
                delta = (u16)(0u - delta);
            }
            distance = (u16)(distance + delta * delta);

            delta = (u16)(blended_blue - *candidate_color++);
            if ((i16)delta < 0) {
                delta = (u16)(0u - delta);
            }
            distance = (u16)(distance + delta * delta);

            if (distance <= best_distance) {
                best_distance = distance;
                best_index = (i16)candidate;
            }
        }
        if (best_index >= 0) {
            *table = (u8)best_index;
        }
        ++table;
    }
#if defined(__WATCOMC__)
    _asm pop es;
#endif
    return negative_percent;
}
