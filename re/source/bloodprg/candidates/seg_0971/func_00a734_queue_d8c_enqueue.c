#include "../include/bloodprg_list.h"

int CB_NEAR queue_d8c_enqueue(cb_u16 byte_count)
{
    list_d8c_head_offset = (cb_u16)(list_d8c_head_offset + byte_count);
    list_d8c_byte_count = (cb_u16)(list_d8c_byte_count + byte_count);
    return 1;
}
