#include <conio.h>
#include <dos.h>
#include <string.h>

#include "../include/bloodprg_hardware.h"

#if defined(__WATCOMC__)
#pragma intrinsic(_fmemset)
#endif

void CB_FAR vga_mode_x_initialize(void)
{
    union REGPACK font_registers;
    union REGS registers;
    cb_u16 port;
    cb_u8 value;

    registers.x.ax = 0x0f00u;
    int86(0x10, &registers, &registers);
    saved_video_mode = registers.h.al;

    registers.x.ax = 0x0013u;
    int86(0x10, &registers, &registers);

    font_registers.w.ax = 0x1130u;
    font_registers.h.bh = 3u;
    intr(0x10, &font_registers);
    bios_font_8x8 = (bloodprg_font_ptr)MK_FP(
            font_registers.w.es, font_registers.w.bp);

    video_crtc_base_port = *(volatile cb_u16 CB_FAR *)MK_FP(0x0040u, 0x0063u);
    vga_dac_clear();

    port = 0x03ceu;
    outp(port, 5u);
    value = (cb_u8)inp(++port);
    outp(port, value & 0xefu);

    --port;
    outp(port, 6u);
    value = (cb_u8)inp(++port);
    outp(port, value & 0xfdu);

    port = 0x03c4u;
    outp(port, 4u);
    value = (cb_u8)inp(++port);
    outp(port, (value & 0xf7u) | 0x04u);

    port = video_crtc_base_port;
    outp(port, 0x14u);
    value = (cb_u8)inp(++port);
    outp(port, value & 0xbfu);

    --port;
    outp(port, 0x17u);
    value = (cb_u8)inp(++port);
    outp(port, value | 0x40u);

    port = video_crtc_base_port;
    outp(port, 0x11u);
    value = (cb_u8)inp(++port);
    outp(port, value | 0x20u);

    outpw(0x03c4u, 0x0f02u);
    _fmemset((void CB_FAR *)MK_FP(0xa000u, 0), 0, 0xffffu);
}
