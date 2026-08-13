#include <conio.h>

typedef unsigned char u8;
typedef unsigned int u16;

typedef volatile u8 far *graphics_buffer_ptr;

extern graphics_buffer_ptr __based(__segname("GAME_DATA"))
        graphics_draw_framebuffer;
extern volatile u16 __based(__segname("GAME_DATA")) ship_3d_depth_offset;
extern volatile u8 __based(__segname("GAME_DATA"))
        ship_3d_plane_blit_crop_enabled;

#define VGA_SEQUENCER_PORT 0x03c4u
#define VGA_MAP_MASK_INDEX 2u
#define VGA_PLANE_COUNT 4u
#define PLANAR_PAGE_BYTES 16000u
#define CROP_DESTINATION_OFFSET 0x0af0u
#define CROP_SOURCE_OFFSET 0x2bc0u
#define CROP_PLANE_BYTES 0x28a0u
#define PLANAR_ROW_BYTES 80u
#define CHUNKY_ROW_BYTES 320u

void far chunky_to_planar_framebuffer(const volatile u8 far *source);
#pragma aux chunky_to_planar_framebuffer parm [ds si] modify exact [dx]

void far chunky_to_planar_framebuffer(const volatile u8 far *source)
{
    const volatile u8 far *plane_source;
    volatile u8 far *destination;
    volatile u8 far *plane_destination;
    u16 byte_count;
    u16 count;
    u16 depth_offset;
    u16 plane;

    _asm push ax;
    _asm push es;
    _asm push ds;
    _asm cld;

    destination = graphics_draw_framebuffer;
    byte_count = PLANAR_PAGE_BYTES;

    if ((ship_3d_plane_blit_crop_enabled & 1u) != 0u) {
        destination += CROP_DESTINATION_OFFSET;
        source += CROP_SOURCE_OFFSET;
        byte_count = CROP_PLANE_BYTES;

        depth_offset = ship_3d_depth_offset;
        if (depth_offset != 0u) {
            destination += (u16)(depth_offset * PLANAR_ROW_BYTES);
            byte_count = (u16)(
                    byte_count
                    - (u16)(depth_offset * 2u) * PLANAR_ROW_BYTES);
            source += (u16)(depth_offset * CHUNKY_ROW_BYTES);
            if (byte_count == 0u) {
                goto restore_registers;
            }
        }
    }

    for (plane = 0u; plane < VGA_PLANE_COUNT; ++plane) {
        outpw(
                VGA_SEQUENCER_PORT,
                (u16)(VGA_MAP_MASK_INDEX | ((u16)(1u << plane) << 8)));
        plane_source = source + plane;
        plane_destination = destination;
        count = byte_count;
        do {
            *plane_destination++ = *plane_source++;
            plane_source += VGA_PLANE_COUNT - 1u;
        } while (--count != 0u);
    }

restore_registers:
    _asm pop ds;
    _asm pop es;
    _asm pop ax;
}
