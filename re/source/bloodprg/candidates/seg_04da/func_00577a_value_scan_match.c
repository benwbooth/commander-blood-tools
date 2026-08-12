#include "../include/bloodprg_vm.h"

const cb_u8 CB_NEAR *CB_NEAR value_scan_match(
    cb_u16 value,
    const bloodprg_value_node CB_NEAR *node
)
{
    while (node != 0) {
        if (node->value == value) {
            return node->payload;
        }
        node = node->next;
    }
    return 0;
}
