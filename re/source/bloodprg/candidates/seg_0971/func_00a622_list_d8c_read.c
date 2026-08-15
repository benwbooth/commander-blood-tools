#include <dos.h>

#include "../include/bloodprg_list.h"

int CB_NEAR list_d8c_read(cb_u16 *entry_extent, cb_u16 *cursor_offset)
{
    cb_u16 cursor;

    if (!ems_paged_read(2u)) {
        return 0;
    }

    cursor = list_d8c_head_offset;
    *cursor_offset = cursor;
    *entry_extent = *(volatile cb_u16 CB_FAR *)
            MK_FP(list_d8c_head_segment, (cb_u16)(cursor - 2u));
    return 1;
}
