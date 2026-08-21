#include <dos.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_platform.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_vm.h"

#if defined(__WATCOMC__)
static bloodprg_vm_image_ptr CB_NEAR vm_script_handler_invoke(
        bloodprg_vm_opcode_handler CB_NEAR *handler,
        bloodprg_vm_image_ptr script_bytes);
#pragma aux vm_script_handler_invoke = \
        "push ds" \
        "push dx" \
        "mov ds,dx" \
        "call bx" \
        "pop dx" \
        "pop ds" \
        parm [bx] [dx ax] value [dx ax] \
        modify exact [ax bx cx si di es]
#else
static bloodprg_vm_image_ptr CB_NEAR vm_script_handler_invoke(
        bloodprg_vm_opcode_handler CB_NEAR *handler,
        bloodprg_vm_image_ptr script_bytes)
{
    return handler(script_bytes);
}
#endif

bloodprg_vm_image_ptr CB_NEAR vm_opcode_dispatch(
        cb_u8 opcode, bloodprg_vm_image_ptr script_bytes)
{
    switch (opcode) {
    case 0xa0u:
        return vm_op_a0_push(script_bytes);
    case 0xa1u:
        return vm_op_a1_pop(script_bytes);
    default:
        return vm_script_handler_invoke(
                vm_opcode_handlers[
                    (cb_u8)(opcode - BLOODPRG_VM_OPCODE_MIN)],
                script_bytes);
    }
}

cb_i16 CB_FAR vm_run_wrapper(void)
{
    bloodprg_resource_resolve_result resolved;
    bloodprg_vm_image_ptr current_resource;
    bloodprg_vm_image_ptr cursor;
    cb_u16 resource_index;
    cb_u8 opcode;
    cb_u8 signal;

    if ((vm_execution_enabled & 1u) == 0u) {
        return 0;
    }

    rtc_time_read();
    rtc_date_read();

    current_resource = (bloodprg_vm_image_ptr)MK_FP(
            FP_SEG((const void CB_FAR *)vm_resource_images), 0u);
    for (resource_index = 0u;
            resource_index < BLOODPRG_VM_RESOURCE_COUNT;
            ++resource_index) {
        resolved = resource_handle_resolve(vm_resource_handles[resource_index]);
        if (resolved.loaded != 0u) {
            current_resource = (bloodprg_vm_image_ptr)MK_FP(
                    resolved.segment, resolved.offset);
        }
        vm_resource_images[resource_index] = current_resource;
    }

    vm_state_processor();
    vm_block_scan_flags = 0u;

    cursor = vm_resource_images[0];
    if ((vm_resume_state & BLOODPRG_VM_RESUME_ACTIVE) != 0u) {
        cursor = (bloodprg_vm_image_ptr)MK_FP(
                FP_SEG(cursor), vm_resume_cursor);
    }

    for (;;) {
        opcode = *cursor++;
        if (opcode == BLOODPRG_VM_STREAM_END) {
            break;
        }

        vm_yield_flag = 0u;
        cursor = vm_opcode_dispatch(opcode, cursor);
        signal = vm_yield_flag;

        if (signal == 0u) {
            if ((vm_skip_count & BLOODPRG_VM_SKIP_COUNT_MASK) != 0u) {
                do {
                    cursor = vm_script_handler_invoke(
                            vm_token_advance, cursor);
                } while (--vm_skip_count != 0u);
            } else if (vm_resume_state == 1u) {
                vm_resume_state = 0u;
                cursor = (bloodprg_vm_image_ptr)MK_FP(
                        FP_SEG(cursor), vm_text_loop_target);
                continue;
            }
        } else {
            vm_presentation_start_lock = 1u;
            if (signal == BLOODPRG_VM_YIELD_CONTINUE) {
                vm_skip_count = 0u;
            } else if (signal == BLOODPRG_VM_YIELD_SAVE_CURSOR) {
                ++vm_resume_state;
                vm_resume_cursor = FP_OFF(cursor);
            } else {
                error_overlay_draw(0u, (const cb_u8 CB_FAR *)cursor);
                return -1;
            }
        }

        if ((vm_resume_state & BLOODPRG_VM_RESUME_ACTIVE) != 0u
                && FP_OFF(cursor) >= vm_text_loop_target) {
            break;
        }
    }

    vm_flag_test_67b1();
    presentation_scan();
    return 0;
}
