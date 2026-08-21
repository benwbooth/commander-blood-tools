#include "../include/bloodprg_vm.h"

bloodprg_vm_image_ptr CB_NEAR vm_op_d1_cond_branch(
        bloodprg_vm_image_ptr script_bytes)
{
    if ((vm_scene_gate_gs & 1u) == 0) {
        return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
    }
    return script_bytes;
}
