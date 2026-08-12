#ifndef XDB_VIDEO_H
#define XDB_VIDEO_H

#include "xdb_common.h"

extern volatile xdb_u16 xdb_video_page_4000; /* DS:0x0026 */
extern volatile xdb_u16 xdb_video_page_a400; /* DS:0x0028 */

#if defined(__WATCOMC__)
extern void XDB_NEAR xdb_port_write_u8(xdb_u16 port, xdb_u8 value);
#pragma aux xdb_port_write_u8 = \
        "out dx,al" \
        parm [dx] [al] \
        modify exact []
extern void XDB_NEAR xdb_port_write_u16(xdb_u16 port, xdb_u16 value);
#pragma aux xdb_port_write_u16 = \
        "out dx,ax" \
        parm [dx] [ax] \
        modify exact []
extern xdb_u8 XDB_NEAR xdb_port_read_u8(xdb_u16 port);
#pragma aux xdb_port_read_u8 = \
        "in al,dx" \
        parm [dx] \
        value [al] \
        modify exact []
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define xdb_port_write_u8(port, value) outportb((port), (value))
#define xdb_port_write_u16(port, value) outport((port), (value))
#define xdb_port_read_u8(port) inportb(port)
#else
void xdb_port_write_u8(xdb_u16 port, xdb_u8 value);
void xdb_port_write_u16(xdb_u16 port, xdb_u16 value);
xdb_u8 xdb_port_read_u8(xdb_u16 port);
#endif

void XDB_NEAR xdb_amer_vga_clear_and_sync(void);
void XDB_NEAR xdb_croolis_vga_clear_and_sync(void);
void XDB_NEAR xdb_scrut_vga_clear_and_sync(void);

#if defined(__WATCOMC__)
#pragma aux xdb_amer_vga_clear_and_sync \
        modify exact [ax bx cx dx di es]
#pragma aux xdb_croolis_vga_clear_and_sync \
        modify exact [ax bx cx dx di es]
#pragma aux xdb_scrut_vga_clear_and_sync \
        modify exact [ax bx cx dx di es]
#endif

#endif
