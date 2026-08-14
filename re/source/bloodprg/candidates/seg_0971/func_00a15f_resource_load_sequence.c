#include "../include/bloodprg_list.h"
#include "../include/bloodprg_resource.h"

#define LIST_D8C_INITIAL_REFILL_COUNT 50u
#define LIST_D8C_SKIP_INITIAL_REFILL 0x0040u

void CB_NEAR resource_load_sequence(cb_u16 resource_id)
{
    volatile cb_u16 CB_FAR *entry;
    cb_u16 entry_extent;
    cb_u16 link_target_offset;
    cb_u16 refill_count;

    if (!resource_switch(resource_id)) {
        return;
    }
    if (!banked_list_load()) {
        return;
    }

    entry = list_d8c_tail_pointer;
    entry_extent = *entry++;
    link_target_offset = list_d8c_default_entry_segment;
    list_d8c_activate_entry(entry_extent, entry, link_target_offset);
    list_d8c_active_present();
    list_d8c_init();

    ++list_d8c_read_wrap_index;
    ++list_d8c_sequence_index;
    ++list_d8c_wrap_count;

    if ((resource_flags & LIST_D8C_SKIP_INITIAL_REFILL) == 0u) {
        for (refill_count = 0u;
                refill_count < LIST_D8C_INITIAL_REFILL_COUNT;
                ++refill_count) {
            link_target_offset = list_d8c_refill(link_target_offset);
        }
    }
    list_d8c_previous_tick = timer_tick_count;
}
