#include "../include/bloodprg_list.h"
#include "../include/bloodprg_vm.h"

void CB_FAR presentation_update_1fb2(void)
{
    if ((vm_c2_presentation_gate & 1u) != 0) {
        presentation_queue_finish();
        if ((vm_ship_active_flags_low & 8u) != 0) {
            vm_bridge_redraw_pending = 1;
        }
        vm_active_line = 0xffffu;
        vm_c2_presentation_gate = 0;
        vm_presentation_request_flags &= (cb_u8)~2u;
    }
}
