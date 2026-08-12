typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned long u32;

#if defined(__WATCOMC__) || defined(__TURBOC__)
#define FAR far
#else
#define FAR
#endif

void FAR composite_draw_a(u8 color, u16 x, u16 y,
        u16 width, u16 height);
void FAR blit_coord_guard_c(u8 color, u16 x, u16 y,
        u16 width, u16 height);
u32 FAR layout_offset_calc_probe(u16 columns, u16 rows);

#if defined(__WATCOMC__)
#pragma aux layout_offset_calc_probe parm [ax] [bx] value [bx ax]
/* Watcom reserves BP, so the fifth helper argument remains stack-passed. */
#pragma aux composite_draw_a parm [ax] [bx] [cx] [dx] modify exact []
#pragma aux blit_coord_guard_c parm [ax] [bx] [cx] [dx] modify exact []
#endif

u32 FAR layout_offset_calc_probe(u16 columns, u16 rows)
{
    u16 width;
    u16 height;
    u16 x;
    u16 y;

    width = (u16)(columns * 4u + 4u);
    height = (u16)(rows * 6u + 4u);
    x = (u16)((320u - width) >> 1);
    y = (u16)((200u - height) >> 1);

    blit_coord_guard_c(0, x, y, width, height);
    composite_draw_a(0x0fu, x, y, width, height);

    return ((u32)(u16)(y + 2u) << 16) | (u16)(x + 2u);
}
