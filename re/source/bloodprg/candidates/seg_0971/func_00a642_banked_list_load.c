#include <dos.h>

#include "../include/bloodprg_list.h"

int CB_NEAR banked_list_load(void)
{
    cb_u16 entry_extent;
    cb_u16 cursor_offset;
    cb_u16 entry_start;

    list_d8c_init();
    if (!list_d8c_read(&entry_extent, &cursor_offset)) {
        return 0;
    }

    entry_start = (cb_u16)(
            list_d8c_buffer_end_offset - entry_extent - 2u);
    list_d8c_tail_offset = entry_start;
    *(volatile cb_u16 CB_FAR *)MK_FP(
            list_d8c_head_segment, entry_start) = entry_extent;
    entry_start += 2u;
    list_d8c_head_offset = entry_start;

    return ems_paged_read((cb_u16)(entry_extent - 2u));
}
