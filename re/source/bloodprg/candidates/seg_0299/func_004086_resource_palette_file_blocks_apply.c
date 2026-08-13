#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_resource.h"

void CB_NEAR resource_palette_file_blocks_apply(cb_u16 file_handle,
        volatile cb_u16 *header_buffer,
        cb_u32 *remaining_bytes)
{
    volatile cb_u8 *destination;
    cb_u16 header;
    cb_u16 byte_count;

    palette_dirty = 1;

    for (;;) {
        (void)cb_dos_read(file_handle,
                (volatile cb_u8 CB_FAR *)header_buffer, 2u);
        header = *header_buffer;
        *remaining_bytes -= 2u;
        if (header == 0xffffu) {
            break;
        }

        destination = live_palette + (cb_u16)((header & 0x00ffu) * 3u);
        byte_count = (cb_u16)((header >> 8) * 3u);
        *remaining_bytes -= byte_count;
        (void)cb_dos_read(file_handle,
                (volatile cb_u8 CB_FAR *)destination, byte_count);
    }
}
