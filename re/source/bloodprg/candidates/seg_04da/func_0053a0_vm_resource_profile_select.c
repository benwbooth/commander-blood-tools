#include <dos.h>

#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_ship3d.h"
#include "../include/bloodprg_vm.h"

#define VM_PROFILE_STATE_WORD_COUNT 128u

cb_i16 CB_FAR vm_resource_profile_select(cb_u16 profile)
{
    const cb_u16 CB_FS_DATA *profile_resource;
    const volatile bloodprg_vm_directory_entry CB_FAR *entry;
    bloodprg_resource_resolve_result state_resource;
    bloodprg_resource_resolve_result directory_resource;
    cb_u16 resource_index;
    cb_u16 object_offset;

    if (profile != vm_resource_profile_index) {
        for (resource_index = 0u;
                resource_index < BLOODPRG_VM_RESOURCE_COUNT;
                ++resource_index) {
            resource_release(vm_resource_handles[resource_index]);
        }
    }

    vm_resource_profile_index = profile;
    profile_resource = vm_resource_profiles[profile];
    for (resource_index = 0u;
            resource_index < BLOODPRG_VM_RESOURCE_COUNT;
            ++resource_index) {
        vm_resource_handles[resource_index] = *profile_resource;
        if (resource_load_by_id(*profile_resource) == 0) {
            return -1;
        }
        ++profile_resource;
    }

    for (resource_index = 0u;
            resource_index < VM_PROFILE_STATE_WORD_COUNT;
            ++resource_index) {
        vm_state_words[resource_index] = 0xffffu;
    }
    for (resource_index = 0u;
            resource_index < BLOODPRG_VM_SPECIAL_SLOT_COUNT;
            ++resource_index) {
        vm_special_slots[resource_index] = 0u;
    }

    vm_profile_cursor = 0u;
    vm_branch_stack_top = 0u;
    vm_query_mode = 0u;
    vm_subtitle_wrap_marker = 0u;
    nav_pending_record_link = 0u;
    vm_presentation_request_flags = 0u;
    vm_execution_enabled = 0u;
    vm_blood_history_ring_index = 0u;
    vm_skip_count = 0u;
    vm_presentation_active = 0u;
    vm_presentation_defer_a = 0u;
    vm_text_display_active = 0u;
    vm_word_choice_active = 0u;
    vm_block_match_value = 0u;
    vm_profile_word_6766 = 0u;
    nav_deferred_record_type = 0u;
    nav_deferred_record_link = 0u;
    vm_profile_word_676e = 0u;
    vm_query_auxiliary = 0u;
    vm_presentation_start_lock = 0u;
    vm_profile_flag_67af = 0u;
    vm_block_scan_flags = 0u;
    vm_resume_state = 0u;
    vm_resume_cursor = 0u;
    vm_text_loop_target = 0u;
    ship_3d_nav_source_offsets[0] = 0u;
    vm_presentation_reg_6770 = 0u;
    vm_profile_word_6786 = 0u;
    vm_branch_a = 0u;
    vm_branch_b = 0u;
    vm_program_counter = 0u;
    vm_parent_program_counter = 0u;
    vm_profile_record_word = 0u;
    vm_c1_related_operand = 0u;
    vm_profile_word_67a0 = 0u;
    vm_profile_word_67a2 = 0u;

    state_resource = resource_handle_resolve(vm_resource_handles[2]);
    directory_resource = resource_handle_resolve(vm_resource_handles[4]);
    entry = (const volatile bloodprg_vm_directory_entry CB_FAR *)MK_FP(
            directory_resource.segment, directory_resource.offset);

    for (;;) {
        if (entry->entry_kind == BLOODPRG_VM_DIRECTORY_ACTIVE_KIND) {
            object_offset = entry->object_offset;
            if (string_compare(entry->name, vm_builtin_name_blood)) {
                vm_wildcard_ref_value = object_offset;
                vm_primary_c4_record =
                        (cb_u16)(object_offset + 8u);
                vm_blood_history_words = (volatile cb_u16 CB_FAR *)MK_FP(
                        state_resource.segment,
                        (cb_u16)(object_offset + 16u));
            } else if (string_compare(entry->name, vm_builtin_name_orxx)) {
                vm_named_orxx_object = object_offset;
            } else if (string_compare(entry->name, vm_builtin_name_honk)) {
                vm_named_honk_object = object_offset;
            } else if (string_compare(entry->name, vm_builtin_name_menu)) {
                vm_named_menu_object = object_offset;
            } else if (string_compare(entry->name, vm_builtin_name_arche)) {
                vm_arche_record_offset = object_offset;
            } else {
                if (string_compare(entry->name, vm_builtin_name_ark)) {
                    vm_named_ark_object = object_offset;
                }
                if (string_compare(entry->name, vm_builtin_name_scruter_jo)) {
                    vm_named_scruter_jo_object = object_offset;
                }
            }
        } else if (entry->entry_kind == 0u) {
            break;
        } else if (entry->entry_kind == 5u
                && string_compare(entry->name, vm_builtin_name_vbio)) {
            vm_named_vbio_object = entry->object_offset;
        }
        ++entry;
    }

    return 0;
}
