/* Codegen probe for BLOODPRG 0x003B45. */

typedef unsigned char u8;
typedef unsigned int u16;

extern void far horizontal_span(u8 color, u16 x, u16 y, u16 width);
extern void far vertical_span(u8 color, u16 x, u16 y, u16 height);

#if defined(__WATCOMC__)
#pragma aux horizontal_span parm [ax] [bx] [cx] [dx] modify exact []
#pragma aux vertical_span parm [ax] [bx] [cx] [dx] modify exact []
#endif

void far composite_draw_a_probe(
        u8 color, u16 x, u16 y, u16 width, u16 height)
{
    horizontal_span(color, x, y, width);
    vertical_span(color, x, y, height);
    vertical_span(color, (u16)(x + width - 1u), y, height);
    horizontal_span(color, x, (u16)(y + height - 1u), width);
}

#if defined(__WATCOMC__)
#pragma aux composite_draw_a_probe parm [ax] [bx] [cx] [dx] modify exact []
#endif
