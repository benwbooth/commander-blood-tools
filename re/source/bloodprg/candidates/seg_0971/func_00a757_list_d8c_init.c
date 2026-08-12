#include "../include/bloodprg_list.h"

void CB_FAR list_d8c_init(void)
{
    cb_u16 base_segment;

    base_segment = list_d8c_base_segment;
    list_d8c_head_segment = base_segment;
    list_d8c_tail_segment = base_segment;

    list_d8c_head_offset = 0;
    list_d8c_tail_offset = 0;
    list_d8c_byte_count = 0;
    list_d8c_iteration_count = 0;
    list_d8c_active_segment = 0;
    list_d8c_wrap_limit = list_d8c_buffer_end_offset;
}
