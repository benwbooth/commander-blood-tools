#include "../include/bloodprg_list.h"

int CB_NEAR queue_d8c_empty_check(cb_u16 byte_count)
{
    cb_u16 head;
    cb_u16 tail;
    cb_u16 needed;

    head = list_d8c_head_offset;
    tail = list_d8c_tail_offset;
    if (head < tail) {
        needed = (cb_u16)(head + byte_count);
        needed = (cb_u16)(needed + 0x12u);
        if (tail < needed) {
            return 0;
        }
    }

    needed = (cb_u16)(list_d8c_byte_count + 0x0au);
    needed = (cb_u16)(needed + byte_count);
    if (needed < byte_count) {
        return 0;
    }

    return list_d8c_wrap_limit >= needed;
}
