#include <conio.h>

typedef unsigned char u8;
typedef unsigned int u16;

#define VGA_GC_PORT 0x03ceu
#define VGA_READ_MAP_INDEX 4u
#define VGA_PLANE_COUNT 4u
#define VGA_PLANE_BYTES 16000u

void far vga_planar_to_chunky(
        const volatile u8 far *source,
        volatile u8 far *destination);
#pragma aux vga_planar_to_chunky parm [ds si] [es di] modify exact []

void far vga_planar_to_chunky(
        const volatile u8 far *source,
        volatile u8 far *destination)
{
    const volatile u8 far *plane_source;
    volatile u8 far *pixel;
    u16 count;
    u16 plane;

    _asm cld;

    for (plane = 0u; plane < VGA_PLANE_COUNT; ++plane) {
        outpw(
                VGA_GC_PORT,
                (u16)(VGA_READ_MAP_INDEX | (plane << 8)));
        plane_source = source;
        pixel = destination + plane;
        count = VGA_PLANE_BYTES;
        do {
            *pixel++ = *plane_source++;
            pixel += VGA_PLANE_COUNT - 1u;
        } while (--count != 0u);
    }
}
