#include "../include/bloodprg_list.h"
#include "../include/bloodprg_resource.h"

void CB_NEAR list_d8c_refill_with_rollover_latch(
        cb_u16 link_target_offset)
{
    list_d8c_rollover_state = (cb_u8)resource_flags & 0x80u;
    list_d8c_refill(link_target_offset);
    list_d8c_rollover_state = 0u;
}
