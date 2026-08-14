#include "../include/bloodprg_vm.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_CONTROL_CODE_AT(base, offset) \
    ((const volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(base), (offset)))
#define VM_CONTROL_FIELD_AT(record, offset) \
    ((volatile cb_u16 CB_FAR *)MK_FP( \
        FP_SEG(record), (cb_u16)(FP_OFF(record) + (offset))))
#else
#define VM_CONTROL_CODE_AT(base, offset) ((base) + (offset))
#define VM_CONTROL_FIELD_AT(record, offset) \
    ((volatile cb_u16 *)((const volatile cb_u8 *)(record) + (offset)))
#endif

void CB_NEAR vm_control_flow(
    const volatile bloodprg_vm_object_header CB_FAR *object,
    cb_u16 code_offset)
{
    bloodprg_vm_image_ptr code_image;
    const bloodprg_value_node CB_FAR *code_nodes;
    const cb_u8 CB_NEAR *matched_block;
    volatile cb_u16 CB_FAR *control_field;
    cb_u16 control_value;
    cb_u16 field_offset;
    cb_u16 match_offset;

    code_image = vm_code_image;
    vm_block_scan_flags = 1;
    ++code_offset;
    vm_pc_saved = code_offset;
    code_nodes = (const bloodprg_value_node CB_FAR *)VM_CONTROL_CODE_AT(
        code_image, code_offset);

    field_offset = (cb_u16)vm_field_offset(0x000fu, object->kind);
    control_field = VM_CONTROL_FIELD_AT(object, field_offset);
    control_value = *control_field;
    if (control_value == 0u) {
        control_value = code_nodes->value;
    }
    if (vm_branch_a != 0u) {
        control_value = vm_branch_a;
    }
    *control_field = control_value;
    vm_branch_a = control_value;

    matched_block = value_scan_match(control_value, code_nodes);
    match_offset = (cb_u16)matched_block;
    if (match_offset != 0u) {
        vm_program_counter = match_offset;
        vm_script_block_scan((bloodprg_vm_image_ptr)VM_CONTROL_CODE_AT(
            code_image, match_offset));
        vm_op_a3_collect();
    }

    if (vm_branch_b != 0u) {
        matched_block = value_scan_match(vm_branch_b, code_nodes);
        match_offset = (cb_u16)matched_block;
        if (match_offset != 0u) {
            vm_script_block_scan((bloodprg_vm_image_ptr)VM_CONTROL_CODE_AT(
                code_image, match_offset));
        }
    }
}
