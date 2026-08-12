#include "../include/xdb_video.h"

void XDB_NEAR xdb_croolis_vga_clear_and_sync(void)
{
    volatile xdb_u16 XDB_FAR *video = XDB_FAR_AT(xdb_u16, 0xa000, 0);
    xdb_u16 count;
    xdb_u8 status;

    xdb_port_write_u8(0x03c8, 0);
    count = 0x0300;
    do {
        xdb_port_write_u8(0x03c9, 0);
    } while (--count != 0);

    xdb_video_page_4000 = 0x4000;
    xdb_video_page_a400 = 0xa400;
    xdb_port_write_u16(0x03d4, 0x000c);
    xdb_port_write_u16(0x03c4, 0x0f02);

    count = 0x7d00;
    do {
        *video++ = 0;
    } while (--count != 0);

    do {
        status = xdb_port_read_u8(0x03da);
    } while ((status & 0x08) != 0);
    do {
        status = xdb_port_read_u8(0x03da);
    } while ((status & 0x08) == 0);
}
