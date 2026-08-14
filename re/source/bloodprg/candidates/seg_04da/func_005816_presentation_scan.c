#include <dos.h>

#include "../include/bloodprg_entity.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_nav.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_vm.h"

#define PRESENTATION_OWNER_ACTIVE 0x0001u
#define PRESENTATION_OWNER_BLOCKED 0x8000u
#define PRESENTATION_RELATED_FLAG20 0x0020u
#define PRESENTATION_RELATED_LATCH 0x8000u
#define PRESENTATION_UI_NAME_LOOKUP 0x0001u
#define PRESENTATION_UI_BUSY 0x0004u
#define PRESENTATION_KIND_ACTOR 0x0001u
#define PRESENTATION_KIND_CHARACTER 0x0002u
#define PRESENTATION_KIND_SHIP 0x0010u
#define PRESENTATION_KIND_SPECIAL 0x0200u
#define PRESENTATION_RECORD_C1 0x00c1u
#define PRESENTATION_RECORD_C4 0x00c4u
#define PRESENTATION_RECORD_C6 0x00c6u
#define PRESENTATION_EFFECT_RESOURCE 0x8007u
#define PRESENTATION_EFFECT_ENTITY 2u
#define PRESENTATION_EFFECT_X 0x0010u
#define PRESENTATION_EFFECT_Y 0x004au
#define PRESENTATION_HISTORY_WORDS 8u

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define PRESENTATION_RECORD_AT(segment, offset, type) \
    ((volatile type CB_FAR *)MK_FP((segment), (cb_u16)(offset)))
#define PRESENTATION_NEXT_ENTRY(entry) \
    ((const volatile bloodprg_vm_directory_entry CB_FAR *)MK_FP( \
        FP_SEG(entry), \
        (cb_u16)(FP_OFF(entry) + sizeof(bloodprg_vm_directory_entry))))
#else
#define PRESENTATION_RECORD_AT(segment, offset, type) \
    ((volatile type CB_FAR *)(vm_record_base_gs + (cb_u16)(offset)))
#define PRESENTATION_NEXT_ENTRY(entry) ((entry) + 1)
#endif

void CB_NEAR presentation_scan(void)
{
    const volatile bloodprg_vm_directory_entry CB_FAR *entry;
    volatile bloodprg_vm_scan_object CB_FAR *object;
    volatile bloodprg_vm_scan_object CB_FAR *related;
    volatile bloodprg_vm_record_triple CB_FAR *record;
    volatile bloodprg_vm_record_triple CB_FAR *primary_record;
    volatile bloodprg_vm_record_triple CB_FAR *deferred_target;
    cb_u16 record_segment;
    cb_u16 object_offset;
    cb_u16 field_offset;
    cb_u16 target_offset;
    cb_u16 history_index;
    int run_action;

    vm_presentation_pair_write_disabled = 0u;
    record_segment = FP_SEG(vm_record_base_gs);
    entry = vm_record_directory_gs;

    for (;;) {
        object_offset = entry->object_offset;
        object = PRESENTATION_RECORD_AT(
                record_segment, object_offset, bloodprg_vm_scan_object);
        run_action = 0;

        if ((object->flags & PRESENTATION_OWNER_ACTIVE) != 0u) {
            field_offset = (cb_u16)vm_field_offset(
                    BLOODPRG_VM_RECIPROCAL_SELECTOR, object->kind);
            record = PRESENTATION_RECORD_AT(
                    record_segment,
                    (cb_u16)(object_offset + field_offset),
                    bloodprg_vm_record_triple);

            switch (object->kind) {
            case PRESENTATION_KIND_CHARACTER:
                if ((vm_presentation_active_gs & 1u) != 0u
                        && (vm_c2_presentation_gate_gs & 1u) == 0u
                        && (vm_presentation_word_choice_active_gs & 1u) == 0u
                        && (vm_presentation_start_lock & 1u) == 0u) {
                    primary_record = PRESENTATION_RECORD_AT(
                            record_segment,
                            vm_presentation_primary_c4_record_gs,
                            bloodprg_vm_record_triple);
                    if (primary_record->kind == PRESENTATION_RECORD_C4
                            && record->kind == PRESENTATION_RECORD_C4
                            && record->related == vm_wildcard_ref_value_gs
                            && (object->flags
                                & PRESENTATION_OWNER_BLOCKED) == 0u) {
                        field_offset = (cb_u16)vm_field_offset(
                                2u, object->kind);
                        target_offset = *PRESENTATION_RECORD_AT(
                                record_segment,
                                (cb_u16)(object_offset + field_offset),
                                cb_u16);
                        if (target_offset != 0u) {
                            vm_control_flow(
                                    (const volatile bloodprg_vm_object_header
                                        CB_FAR *)object,
                                    target_offset);
                        }
                    }
                }
                run_action = 1;
                break;

            case PRESENTATION_KIND_SHIP:
            case PRESENTATION_KIND_SPECIAL:
                run_action = 1;
                break;

            case PRESENTATION_KIND_ACTOR:
                if (record->kind == PRESENTATION_RECORD_C4) {
                    related = PRESENTATION_RECORD_AT(
                            record_segment,
                            record->related,
                            bloodprg_vm_scan_object);
                    vm_presentation_related_flag20 = (cb_u8)(
                            (related->flags & PRESENTATION_RELATED_FLAG20)
                            != 0u);

                    if ((vm_presentation_active_gs & 1u) == 0u) {
                        pbm_palette_dirty = 1u;
                        vm_presentation_status_word = 1u;
                        vm_presentation_active_gs = 1u;
                        vm_branch_a = 0u;
                        vm_branch_b = 0u;
                        vm_pc_saved = 0u;
                        vm_presentation_word_buffer_gs[0] = 0u;
                        vm_presentation_input_gate = 0u;
                        vm_presentation_text_wait_gs = 0u;
                        vm_presentation_word_choice_active_gs = 0u;
                        vm_presentation_hold_ready_gs = 0u;
                        vm_dialogue_hold_complete_gs = 0u;
                        vm_presentation_owner_offset_gs = 0u;
                        vm_presentation_start_lock = 1u;
                        vm_ui_state_gs.word |= PRESENTATION_UI_BUSY;
                        related->flags |= PRESENTATION_RELATED_LATCH;
                        render_update_flag_2751_gs &= 0x7fu;

                        if ((vm_ui_state_gs.word
                                & PRESENTATION_UI_NAME_LOOKUP) != 0u) {
                            (void)vm_c2_descript_lookup(related->name);
                            if ((name_area_effect_active_gs & 1u) != 0u) {
                                name_area_effect_restart_gs = 1u;
                                (void)resource_named_file_load(
                                        PRESENTATION_EFFECT_RESOURCE,
                                        nav_presentation_resource_buffer_gs);
                                entity_record_setter(
                                        PRESENTATION_EFFECT_ENTITY,
                                        nav_presentation_resource_buffer_gs,
                                        PRESENTATION_EFFECT_X,
                                        PRESENTATION_EFFECT_Y,
                                        0u);
                            }
                        }
                    }
                } else if ((vm_presentation_active_gs & 1u) != 0u) {
                    vm_presentation_status_word = 1u;
                    vm_branch_a = 0u;
                    vm_branch_b = 0u;
                    vm_resume_state_gs = 0u;
                    vm_presentation_active_gs = 0u;
                    vm_block_match_value_gs = 0u;
                    vm_ui_state_gs.word &= (cb_u16)~PRESENTATION_UI_BUSY;
                    vm_presentation_request_flags_gs &= 0xfcu;
                    vm_presentation_word_buffer_gs[0] = 0u;
                    vm_presentation_start_lock = 0u;
                    name_area_effect_active_gs = 0u;
                    entity_flag_state_transition(4u);
                    entity_flag_state_transition(2u);
                    for (history_index = 0u;
                            history_index < PRESENTATION_HISTORY_WORDS;
                            ++history_index) {
                        vm_blood_history_words[history_index] = 0u;
                    }
                }

                if (vm_deferred_record_related_gs != 0u
                        && vm_deferred_record_type_gs != 0u) {
                    if (vm_deferred_record_type_gs == PRESENTATION_RECORD_C1
                            || vm_deferred_record_type_gs
                                == PRESENTATION_RECORD_C6) {
                        field_offset = (cb_u16)vm_field_offset(
                                BLOODPRG_VM_RECIPROCAL_SELECTOR,
                                PRESENTATION_KIND_SHIP);
                        deferred_target = PRESENTATION_RECORD_AT(
                                record_segment,
                                (cb_u16)(vm_arche_record_offset_gs
                                    + field_offset),
                                bloodprg_vm_record_triple);
                        deferred_target->kind = vm_deferred_record_type_gs;
                        deferred_target->related =
                                vm_deferred_record_related_gs;
                        deferred_target->value = 0u;
                    } else {
                        record->kind = vm_deferred_record_type_gs;
                        record->related = vm_deferred_record_related_gs;
                        record->value = vm_deferred_record_value_gs;
                    }
                    vm_deferred_record_type_gs = 0u;
                    vm_deferred_record_related_gs = 0u;
                    vm_deferred_record_value_gs = 0u;
                }
                run_action = 1;
                break;
            }

            if (run_action
                    && (cb_i16)record->value >= 0
                    && record->kind != 0u) {
                record_c1_ship3d_action(object, record);
            }
        }

        entry = PRESENTATION_NEXT_ENTRY(entry);
        if (entry->entry_kind != BLOODPRG_VM_DIRECTORY_ACTIVE_KIND) {
            break;
        }
    }
}
