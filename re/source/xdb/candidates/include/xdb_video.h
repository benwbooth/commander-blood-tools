#ifndef XDB_VIDEO_H
#define XDB_VIDEO_H

#include "xdb_common.h"

typedef union xdb_video_page {
    xdb_u16 word;
    struct {
        xdb_u8 low;
        xdb_u8 high;
    } byte;
} xdb_video_page;

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
extern void XDB_NEAR xdb_port_write_buffer_u8(
        xdb_u16 port,
        const volatile xdb_u8 XDB_NEAR *source,
        xdb_u16 count);
#pragma aux xdb_port_write_buffer_u8 = \
        "rep outsb" \
        parm [dx] [si] [cx] \
        modify exact [cx si]
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define xdb_port_write_u8(port, value) outportb((port), (value))
#define xdb_port_write_u16(port, value) outport((port), (value))
#define xdb_port_read_u8(port) inportb(port)
#define xdb_port_write_buffer_u8(port, source, count) \
    do { \
        xdb_u16 xdb_port_buffer_index; \
        for (xdb_port_buffer_index = 0u; \
             xdb_port_buffer_index != (count); \
             ++xdb_port_buffer_index) { \
            outportb((port), (source)[xdb_port_buffer_index]); \
        } \
    } while (0)
#else
void xdb_port_write_u8(xdb_u16 port, xdb_u8 value);
void xdb_port_write_u16(xdb_u16 port, xdb_u16 value);
xdb_u8 xdb_port_read_u8(xdb_u16 port);
void xdb_port_write_buffer_u8(
        xdb_u16 port,
        const volatile xdb_u8 *source,
        xdb_u16 count);
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
