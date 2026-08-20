#include <conio.h>
#include <dos.h>
#include <stdio.h>

#include "bloodprg_graphics.h"
#include "bloodprg_ship3d.h"

#define VGA_SEGMENT 0xa000u
#define VGA_GRAPHICS_INDEX_PORT 0x03ceu
#define VGA_SEQUENCER_INDEX_PORT 0x03c4u
#define CHUNKY_BYTES 64000u
#define PLANE_BYTES 16000u

bloodprg_graphics_buffer_ptr CB_GAME_DATA graphics_draw_framebuffer;
volatile cb_u16 CB_GAME_DATA ship_3d_depth_offset;
volatile cb_u8 CB_GAME_DATA ship_3d_plane_blit_crop_enabled;

static int write_result(const char *text)
{
    FILE *result = fopen("RESULT.TXT", "wt");

    if (result == NULL) {
        return 1;
    }
    fputs(text, result);
    fputc('\n', result);
    fclose(result);
    return text[0] == 'P' ? 0 : 1;
}

static void set_video_mode(cb_u8 mode)
{
    union REGS registers;

    registers.x.ax = mode;
    int86(0x10, &registers, &registers);
}

static void configure_mode_x(void)
{
    cb_u16 port;
    cb_u8 value;

    set_video_mode(0x13u);

    port = VGA_GRAPHICS_INDEX_PORT;
    outp(port, 5u);
    value = (cb_u8)inp(++port);
    outp(port, value & 0xefu);

    --port;
    outp(port, 6u);
    value = (cb_u8)inp(++port);
    outp(port, value & 0xfdu);

    port = VGA_SEQUENCER_INDEX_PORT;
    outp(port, 4u);
    value = (cb_u8)inp(++port);
    outp(port, (value & 0xf7u) | 0x04u);
}

int main(void)
{
    volatile cb_u8 CB_FAR *source;
    volatile cb_u8 CB_FAR *vga;
    cb_u16 source_segment;
    cb_u16 offset;
    cb_u16 plane;
    cb_u8 expected;

    if (_dos_allocmem(0x1000u, &source_segment) != 0u) {
        return write_result("FAIL source allocation");
    }
    source = (volatile cb_u8 CB_FAR *)MK_FP(source_segment, 0u);
    for (offset = 0u; offset < CHUNKY_BYTES; ++offset) {
        source[offset] = (cb_u8)(offset ^ (offset >> 8) ^ (offset * 13u));
    }

    configure_mode_x();
    graphics_draw_framebuffer =
            (bloodprg_graphics_buffer_ptr)MK_FP(VGA_SEGMENT, 0u);
    ship_3d_plane_blit_crop_enabled = 0u;
    ship_3d_depth_offset = 0u;
    chunky_to_planar_framebuffer(source);

    vga = (volatile cb_u8 CB_FAR *)MK_FP(VGA_SEGMENT, 0u);
    for (plane = 0u; plane < 4u; ++plane) {
        outpw(VGA_GRAPHICS_INDEX_PORT, (cb_u16)(4u | (plane << 8)));
        for (offset = 0u; offset < PLANE_BYTES; ++offset) {
            expected = source[(cb_u16)(offset * 4u + plane)];
            if (vga[offset] != expected) {
                set_video_mode(0x03u);
                _dos_freemem(source_segment);
                return write_result("FAIL chunky-to-planar VGA bytes");
            }
        }
    }

    set_video_mode(0x03u);
    _dos_freemem(source_segment);
    return write_result("PASS bloodprg chunky-to-planar VGA bytes");
}
