/* Codegen probe for BLOODPRG 0x000C26. */

#include <conio.h>
#include <dos.h>
#include <string.h>

typedef unsigned char u8;
typedef unsigned int u16;
typedef unsigned long u32;
typedef const u8 far *font_ptr;

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA far
#endif

extern volatile u8 GAME_DATA saved_video_mode;
extern volatile u16 GAME_DATA crtc_base_port;
extern font_ptr GAME_DATA bios_font_8x8;
extern void far vga_dac_clear(void);
extern u32 near bios_font_8x8_get(void);

#if defined(__WATCOMC__)
#pragma intrinsic(_fmemset)
#pragma aux bios_font_8x8_get = \
        "mov ax,1130h" "mov bh,3" "int 10h" "mov ax,bp" "mov dx,es" \
        value [dx ax] modify exact [ax bx dx es bp]
#endif

void far vga_mode_x_initialize_probe(void)
{
    union font_address {
        u32 packed;
        font_ptr pointer;
    } font;
    union REGS registers;
    u16 port;
    u8 value;

    registers.x.ax = 0x0f00u;
    int86(0x10, &registers, &registers);
    saved_video_mode = registers.h.al;
    registers.x.ax = 0x0013u;
    int86(0x10, &registers, &registers);

    font.packed = bios_font_8x8_get();
    bios_font_8x8 = font.pointer;
    crtc_base_port = *(volatile u16 far *)MK_FP(0x0040u, 0x0063u);
    vga_dac_clear();

    port = 0x03ceu;
    outp(port, 5u);
    value = (u8)inp(++port);
    outp(port, value & 0xefu);
    --port;
    outp(port, 6u);
    value = (u8)inp(++port);
    outp(port, value & 0xfdu);

    port = 0x03c4u;
    outp(port, 4u);
    value = (u8)inp(++port);
    outp(port, (value & 0xf7u) | 0x04u);

    port = crtc_base_port;
    outp(port, 0x14u);
    value = (u8)inp(++port);
    outp(port, value & 0xbfu);
    --port;
    outp(port, 0x17u);
    value = (u8)inp(++port);
    outp(port, value | 0x40u);

    port = crtc_base_port;
    outp(port, 0x11u);
    value = (u8)inp(++port);
    outp(port, value | 0x20u);

    outpw(0x03c4u, 0x0f02u);
    _fmemset((void far *)MK_FP(0xa000u, 0), 0, 0xffffu);
}
