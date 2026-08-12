#include "../include/bloodprg_list.h"

void CB_NEAR queue_d8c_enqueue(cb_u16 byte_count)
{
    list_d8c_head_offset += byte_count;
    list_d8c_byte_count += byte_count;
}
