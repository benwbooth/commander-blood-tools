#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_list.h"

volatile cb_u8 CB_FAR *CB_NEAR resource_palette_blocks_apply(
        volatile cb_u8 CB_FAR *stream)
{
    volatile cb_u8 CB_FAR *cursor;
    volatile cb_u8 CB_FAR *start;
    volatile cb_u8 *destination;
    cb_u16 consumed;
    cb_u16 header;
    cb_u16 byte_count;
    cb_u16 remaining;

    start = stream;
    cursor = stream;
    palette_dirty = 1;

    for (;;) {
        header = *(volatile cb_u16 CB_FAR *)cursor;
        cursor += 2;
        if (header == 0xffffu) {
            break;
        }

        destination = live_palette + (cb_u16)((header & 0x00ffu) * 3u);
        byte_count = (cb_u16)((header >> 8) * 3u);
        while (byte_count != 0) {
            *destination++ = *cursor++;
            --byte_count;
        }
    }

    flag_gated_2751();
    if (list_d8c_read_wrap_index == 0) {
        consumed = (cb_u16)(cursor - start);
        remaining = (cb_u16)(list_d8c_entry_metric - consumed);
        list_d8c_entry_metric = (cb_u16)((remaining >> 2) - 2u);
    }

    return cursor;
}
