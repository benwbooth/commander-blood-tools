#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_VALUE_NODE_OFFSET(node) FP_OFF(node)
#define VM_VALUE_NODE_AT(node, offset) \
    ((const bloodprg_value_node CB_FAR *)MK_FP(FP_SEG(node), (offset)))
#else
#define VM_VALUE_NODE_OFFSET(node) ((cb_u16)(unsigned long)(node))
#define VM_VALUE_NODE_AT(node, offset) \
    ((const bloodprg_value_node *)((const cb_u8 *)(node) + (offset)))
#endif

const cb_u8 CB_NEAR *CB_NEAR value_scan_match(
    cb_u16 value,
    const bloodprg_value_node CB_FAR *node
)
{
    cb_u16 result;

    for (;;) {
        if (node->value == value) {
            result = VM_VALUE_NODE_OFFSET(node) + 4u;
            break;
        }
        result = node->next_offset;
        if (result == 0u) {
            break;
        }
        node = VM_VALUE_NODE_AT(node, result);
    }
    return (const cb_u8 CB_NEAR *)result;
}
