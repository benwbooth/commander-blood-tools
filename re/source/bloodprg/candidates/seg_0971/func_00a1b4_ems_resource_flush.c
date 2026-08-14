#include "../include/bloodprg_list.h"
#include "../include/bloodprg_resource.h"

void CB_NEAR ems_resource_flush(cb_u16 link_target_offset)
{
    for (;;) {
        if ((resource_source_is_banked & 1u) == 0u) {
            if (list_d8c_file_handle == 0u) {
                list_d8c_rollover_state = 0u;
                return;
            }
            if ((cb_i8)(cb_u8)resource_flags < 0) {
                list_d8c_refill_with_rollover_latch(link_target_offset);
                return;
            }
        }

        if (!list_d8c_activate_ready()) {
            link_target_offset = list_d8c_refill(link_target_offset);
            continue;
        }

        if (list_d8c_advance_due()) {
            if (list_d8c_palette_offset != 0xFFFFu) {
                (void)list_d8c_palette_blocks_apply();
            }
            list_d8c_active_present();
            queue_d8c_consume();
        }
        list_d8c_refill_with_rollover_latch(link_target_offset);
        return;
    }
}
