/* Codegen probe for the shared XDB VGA clear-and-sync helper. */
#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;

extern volatile u16 video_page_4000;
extern volatile u16 video_page_a400;

#if defined(__WATCOMC__)
extern void near port_write_u8(u16 port, u8 value);
#pragma aux port_write_u8 = \
        "out dx,al" \
        parm [dx] [al] \
        modify exact []
extern void near port_write_u16(u16 port, u16 value);
#pragma aux port_write_u16 = \
        "out dx,ax" \
        parm [dx] [ax] \
        modify exact []
extern u8 near port_read_u8(u16 port);
#pragma aux port_read_u8 = \
        "in al,dx" \
        parm [dx] \
        value [al] \
        modify exact []
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define port_write_u8(port, value) outportb((port), (value))
#define port_write_u16(port, value) outport((port), (value))
#define port_read_u8(port) inportb(port)
#endif

void near xdb_vga_clear_and_sync_probe(void);
#if defined(__WATCOMC__)
#pragma aux xdb_vga_clear_and_sync_probe \
        modify exact [ax bx cx dx di es]
#endif

void near xdb_vga_clear_and_sync_probe(void)
{
    volatile u16 far *video = (volatile u16 far *)MK_FP(0xa000, 0);
    u16 count;
    u8 status;

    port_write_u8(0x03c8, 0);
    count = 0x0300;
    do {
        port_write_u8(0x03c9, 0);
    } while (--count != 0);

    video_page_4000 = 0x4000;
    video_page_a400 = 0xa400;
    port_write_u16(0x03d4, 0x000c);
    port_write_u16(0x03c4, 0x0f02);

    count = 0x7d00;
    do {
        *video++ = 0;
    } while (--count != 0);

    do {
        status = port_read_u8(0x03da);
    } while ((status & 0x08) != 0);
    do {
        status = port_read_u8(0x03da);
    } while ((status & 0x08) == 0);
}
