#include <conio.h>

#include "../include/bloodprg_graphics.h"

#define BLOODPRG_VGA_GC_PORT 0x03ceu
#define BLOODPRG_VGA_READ_MAP_INDEX 4u
#define BLOODPRG_VGA_PLANE_COUNT 4u
#define BLOODPRG_VGA_PLANE_BYTES 16000u

void CB_FAR vga_planar_to_chunky(
        const volatile cb_u8 CB_FAR *source,
        volatile cb_u8 CB_FAR *destination)
{
    const volatile cb_u8 CB_FAR *plane_source;
    volatile cb_u8 CB_FAR *pixel;
    cb_u16 count;
    cb_u16 plane;

#if defined(__WATCOMC__)
    _asm cld;
#endif

    for (plane = 0u; plane < BLOODPRG_VGA_PLANE_COUNT; ++plane) {
        outpw(
                BLOODPRG_VGA_GC_PORT,
                (cb_u16)(BLOODPRG_VGA_READ_MAP_INDEX | (plane << 8)));
        plane_source = source;
        pixel = destination + plane;
        count = BLOODPRG_VGA_PLANE_BYTES;
        do {
            *pixel++ = *plane_source++;
            pixel += BLOODPRG_VGA_PLANE_COUNT - 1u;
        } while (--count != 0u);
    }
}
