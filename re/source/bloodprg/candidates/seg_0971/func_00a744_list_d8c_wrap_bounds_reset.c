#include "../include/bloodprg_list.h"

void CB_NEAR list_d8c_wrap_bounds_reset(void)
{
    list_d8c_wrap_count = 0;
    list_d8c_read_wrap_limit = 0xffffu;
    list_d8c_secondary_wrap_limit = 0xffffu;
}
