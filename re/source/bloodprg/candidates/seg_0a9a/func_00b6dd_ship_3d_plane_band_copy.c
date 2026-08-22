#include <conio.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_ship3d.h"

#if defined(__WATCOMC__)
#pragma intrinsic(inp, outp, outpw)
#endif

#define SHIP_3D_BAND_ROW_BYTES 80u
#define SHIP_3D_BAND_DEPTH_BIAS 35u
#define SHIP_3D_BAND_SOURCE_SPLIT 0xdf40u
#define SHIP_3D_BAND_DESTINATION_SPLIT 0x3e80u
#define SHIP_3D_VGA_SEQUENCER_PORT 0x03c4u
#define SHIP_3D_VGA_ALL_PLANES 0x0f02u
#define SHIP_3D_VGA_GRAPHICS_INDEX_PORT 0x03ceu
#define SHIP_3D_VGA_GRAPHICS_DATA_PORT 0x03cfu
#define SHIP_3D_VGA_MODE_INDEX 5u
#define SHIP_3D_VGA_WRITE_MODE_ONE 1u

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define SHIP_3D_BAND_AT(buffer, offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(buffer), (offset)))
#define SHIP_3D_BAND_DESTINATION_AT(buffer, delta) \
    SHIP_3D_BAND_AT((buffer), (cb_u16)(FP_OFF(buffer) + (delta)))
#else
#define SHIP_3D_BAND_AT(buffer, offset) ((buffer) + (offset))
#define SHIP_3D_BAND_DESTINATION_AT(buffer, delta) ((buffer) + (delta))
#endif

void CB_FAR ship_3d_plane_band_copy(void)
{
    volatile cb_u8 CB_FAR *framebuffer;
    volatile cb_u8 CB_FAR *first_source;
    volatile cb_u8 CB_FAR *second_source;
    volatile cb_u8 CB_FAR *second_destination;
    cb_u16 depth;
    cb_u16 doubled_depth;
    cb_u16 byte_count;
    cb_u16 byte_index;
    cb_u8 graphics_mode;

    if ((ship_3d_plane_blit_crop_enabled_ds & 1u) == 0u) {
        return;
    }

    depth = ship_3d_depth_offset_ds;
    if (palette_transition_increment != 10u) {
        doubled_depth = (cb_u16)(depth + depth);
        if ((cb_i16)doubled_depth > 100) {
            doubled_depth = 100u;
        }
        palette_transition_percent = (cb_u16)(100u - doubled_depth);
    }

    outpw(SHIP_3D_VGA_SEQUENCER_PORT, SHIP_3D_VGA_ALL_PLANES);
    framebuffer = graphics_draw_framebuffer;
    byte_count = (cb_u16)(
        (cb_u16)(cb_u8)(depth + SHIP_3D_BAND_DEPTH_BIAS)
        * SHIP_3D_BAND_ROW_BYTES);
    first_source = SHIP_3D_BAND_AT(
        framebuffer, (cb_u16)(SHIP_3D_BAND_SOURCE_SPLIT - byte_count));
    second_source = SHIP_3D_BAND_AT(framebuffer, SHIP_3D_BAND_SOURCE_SPLIT);
    second_destination = SHIP_3D_BAND_DESTINATION_AT(
        framebuffer,
        (cb_u16)(SHIP_3D_BAND_DESTINATION_SPLIT - byte_count));

    outp(SHIP_3D_VGA_GRAPHICS_INDEX_PORT, SHIP_3D_VGA_MODE_INDEX);
    graphics_mode = (cb_u8)inp(SHIP_3D_VGA_GRAPHICS_DATA_PORT);
    outp(SHIP_3D_VGA_GRAPHICS_DATA_PORT,
        (cb_u8)((graphics_mode & 0xfcu) | SHIP_3D_VGA_WRITE_MODE_ONE));
    for (byte_index = 0u; byte_index < byte_count; ++byte_index) {
        framebuffer[byte_index] = first_source[byte_index];
    }
    for (byte_index = 0u; byte_index < byte_count; ++byte_index) {
        second_destination[byte_index] = second_source[byte_index];
    }
    outp(SHIP_3D_VGA_GRAPHICS_DATA_PORT, graphics_mode);
}
