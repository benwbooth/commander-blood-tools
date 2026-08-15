#include "../include/bloodprg_list.h"

void CB_NEAR queue_d8c_consume(void)
{
    cb_u16 entry_bytes;
    cb_u16 after_header;
    cb_u16 candidate;
    cb_u16 next_tail;
    cb_u16 next_index;

    entry_bytes = *list_d8c_tail_pointer;
    list_d8c_byte_count = (cb_u16)(list_d8c_byte_count - entry_bytes);

    after_header = (cb_u16)(list_d8c_tail_offset + 2u);
    candidate = (cb_u16)(after_header + entry_bytes);
    if (candidate < after_header || candidate > list_d8c_buffer_end_offset) {
        next_tail = (cb_u16)(entry_bytes - 2u);
    } else {
        next_tail = (cb_u16)(list_d8c_tail_offset + entry_bytes);
    }
    list_d8c_tail_offset = next_tail;

    ++list_d8c_sequence_index;
    next_index = (cb_u16)(list_d8c_read_wrap_index + 1u);
    if (next_index > list_d8c_read_wrap_limit) {
        next_index = 1u;
        list_d8c_read_wrap_limit = 0xffffu;
    }
    list_d8c_read_wrap_index = next_index;
}
