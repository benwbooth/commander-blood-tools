#include "../include/bloodprg_list.h"

cb_u16 CB_NEAR queue_d8c_wrap(cb_u16 cursor, cb_u16 byte_count)
{
    cb_u16 next;

    next = (cb_u16)(cursor + byte_count);
    if (next < cursor || next > list_d8c_buffer_end_offset) {
        cb_u16 old_head;

        old_head = list_d8c_head_offset;
        list_d8c_head_offset = 0;
        list_d8c_wrap_limit = old_head;
    }

    list_d8c_iteration_count = (cb_u16)(byte_count - 2u);
    ++list_d8c_wrap_count;
    return next;
}
