#include <conio.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_ship3d.h"

#define BLOODPRG_VGA_SEQUENCER_PORT 0x03c4u
#define BLOODPRG_VGA_MAP_MASK_INDEX 2u
#define BLOODPRG_VGA_PLANE_COUNT 4u
#define BLOODPRG_PLANAR_PAGE_BYTES 16000u
#define BLOODPRG_CROP_DESTINATION_OFFSET 0x0af0u
#define BLOODPRG_CROP_SOURCE_OFFSET 0x2bc0u
#define BLOODPRG_CROP_PLANE_BYTES 0x28a0u
#define BLOODPRG_PLANAR_ROW_BYTES 80u
#define BLOODPRG_CHUNKY_ROW_BYTES 320u

void CB_FAR chunky_to_planar_framebuffer(
        const volatile cb_u8 CB_FAR *source)
{
    const volatile cb_u8 CB_FAR *plane_source;
    volatile cb_u8 CB_FAR *destination;
    volatile cb_u8 CB_FAR *plane_destination;
    cb_u16 byte_count;
    cb_u16 count;
    cb_u16 depth_offset;
    cb_u16 plane;

#if defined(__WATCOMC__)
    _asm push ax;
    _asm push es;
    _asm push ds;
    _asm cld;
#endif

    destination = graphics_draw_framebuffer;
    byte_count = BLOODPRG_PLANAR_PAGE_BYTES;

    if ((ship_3d_plane_blit_crop_enabled & 1u) != 0u) {
        destination += BLOODPRG_CROP_DESTINATION_OFFSET;
        source += BLOODPRG_CROP_SOURCE_OFFSET;
        byte_count = BLOODPRG_CROP_PLANE_BYTES;

        depth_offset = ship_3d_depth_offset;
        if (depth_offset != 0u) {
            destination += (cb_u16)(
                    depth_offset * BLOODPRG_PLANAR_ROW_BYTES);
            byte_count = (cb_u16)(
                    byte_count
                    - (cb_u16)(depth_offset * 2u)
                            * BLOODPRG_PLANAR_ROW_BYTES);
            source += (cb_u16)(
                    depth_offset * BLOODPRG_CHUNKY_ROW_BYTES);
            if (byte_count == 0u) {
                goto restore_registers;
            }
        }
    }

    for (plane = 0u; plane < BLOODPRG_VGA_PLANE_COUNT; ++plane) {
        outpw(
                BLOODPRG_VGA_SEQUENCER_PORT,
                (cb_u16)(
                        BLOODPRG_VGA_MAP_MASK_INDEX
                        | ((cb_u16)(1u << plane) << 8)));
        plane_source = source + plane;
        plane_destination = destination;
        count = byte_count;
        do {
            *plane_destination++ = *plane_source++;
            plane_source += BLOODPRG_VGA_PLANE_COUNT - 1u;
        } while (--count != 0u);
    }

restore_registers:
#if defined(__WATCOMC__)
    _asm pop ds;
    _asm pop es;
    _asm pop ax;
#endif
}
