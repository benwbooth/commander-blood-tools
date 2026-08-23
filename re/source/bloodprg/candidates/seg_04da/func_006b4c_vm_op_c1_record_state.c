#include "../include/bloodprg_input.h"
#include "../include/bloodprg_ship3d.h"

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#include <dos.h>
#define VM_C1_RECORD_AT(base, offset) \
    ((volatile cb_u8 CB_FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define VM_C1_RECORD_AT(base, offset) ((base) + (offset))
#endif

#define VM_C1_RECORD_KIND 0x00c1u
#define VM_C1_RECORD_VALUE 0x0002u
#define VM_C1_OWNER_ACTIVE_FLAG 0x01u
#define VM_C1_OPERAND_SOURCE_FLAG 0x02u
#define VM_C1_NAV_TARGET_KIND 0x0010u
#define VM_C1_SOURCE_FLAG_KIND 0x0001u
#define VM_C1_SOURCE_BITSET_KIND 0x0002u
#define VM_C1_PARENT_SELECTOR 0x0011u
#define VM_C1_DESTINATION_SELECTOR 0x0013u

bloodprg_vm_image_ptr CB_NEAR vm_op_c1_record_state(
        bloodprg_vm_image_ptr script_bytes)
{
    int inverted;
    int matches;
    cb_u16 record_offset;
    cb_u16 owner_offset;
    cb_u16 operand_offset;
    cb_u16 field_offset;
    cb_u16 source_offset;
    volatile cb_u8 CB_FAR *record_base;
    volatile bloodprg_vm_object_header CB_FAR *owner;
    volatile bloodprg_vm_object_header CB_FAR *target;
    volatile bloodprg_vm_object_header CB_FAR *source;
    volatile bloodprg_vm_record_triple CB_FAR *record;
    volatile cb_u16 CB_GAME_DATA *source_cursor;

    record_base = vm_record_base_gs;
    inverted = 0;
    if (*script_bytes == BLOODPRG_VM_OPTION_PREFIX) {
        inverted = 1;
        ++script_bytes;
    }

    record_offset = *(const volatile cb_u16 CB_FAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    owner_offset = vm_record_lookup_by_threshold(record_offset);
    operand_offset = *(const volatile cb_u16 CB_FAR *)script_bytes;
    script_bytes += sizeof(cb_u16);
    vm_c1_related_operand_gs = operand_offset;

    owner = (volatile bloodprg_vm_object_header CB_FAR *)VM_C1_RECORD_AT(
        record_base, owner_offset);
    record = (volatile bloodprg_vm_record_triple CB_FAR *)VM_C1_RECORD_AT(
        record_base, record_offset);

    if ((vm_query_mode_gs & 1u) != 0u) {
        if ((operand_offset == 1u || operand_offset == 2u)
                && record->kind != VM_C1_RECORD_KIND) {
            field_offset = (cb_u16)vm_field_offset(
                VM_C1_PARENT_SELECTOR, operand_offset);
            target = (volatile bloodprg_vm_object_header CB_FAR *)
                VM_C1_RECORD_AT(record_base,
                    *(volatile cb_u16 CB_FAR *)VM_C1_RECORD_AT(
                        record_base, (cb_u16)(owner_offset + field_offset)));
            field_offset = (cb_u16)vm_field_offset(
                VM_C1_DESTINATION_SELECTOR, target->kind);
            if (field_offset == 0u) {
                matches = 0;
            } else {
                record = (volatile bloodprg_vm_record_triple CB_FAR *)
                    ((volatile cb_u8 CB_FAR *)target + field_offset);
                matches = record->kind == VM_C1_RECORD_KIND
                    && record->related == operand_offset;
            }
        } else {
            matches = record->kind == VM_C1_RECORD_KIND
                && record->related == operand_offset;
        }

        if (matches == inverted) {
            return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
        }
        /* The shipped success jumps past the saved SI/DS restores. Natural C
         * keeps the intended successful return instead of that stack defect. */
        return script_bytes;
    }

    if ((owner->flags & VM_C1_OWNER_ACTIVE_FLAG) == 0u) {
        return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
    }

    target = owner;
    if (operand_offset == 1u || operand_offset == 2u) {
        /* The binary clears DL but inherits DH here. The game-level value is
         * the zero-extended A1 flag used by the position resolver. */
        if (ship_3d_position_distance(
                (const volatile bloodprg_vm_object_header CB_FAR *)
                    VM_C1_RECORD_AT(record_base, operand_offset),
                (const volatile bloodprg_vm_object_header CB_FAR *)
                    VM_C1_RECORD_AT(record_base, owner_offset),
                (cb_u16)inverted) != 0u) {
            field_offset = (cb_u16)vm_field_offset(
                VM_C1_PARENT_SELECTOR, owner->kind);
            owner_offset = *(volatile cb_u16 CB_FAR *)VM_C1_RECORD_AT(
                record_base, (cb_u16)(owner_offset + field_offset));
            target = (volatile bloodprg_vm_object_header CB_FAR *)
                VM_C1_RECORD_AT(record_base, owner_offset);
            if (target->kind != VM_C1_NAV_TARGET_KIND) {
                return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
            }
        }
    }

    if (target->kind == VM_C1_NAV_TARGET_KIND) {
        ship_3d_nav_source_list_build_full(
            target, ship_3d_nav_source_offsets);
        source_cursor = ship_3d_nav_source_offsets;
        for (;;) {
            source_offset = *source_cursor++;
            if (source_offset == 0xffffu) {
                /* This original path has the same skipped SI/DS restores as a
                 * successful query; the logical result is success/no write. */
                return script_bytes;
            }

            source = (volatile bloodprg_vm_object_header CB_FAR *)
                VM_C1_RECORD_AT(record_base, source_offset);
            if (source->kind == VM_C1_SOURCE_BITSET_KIND) {
                if (ship_3d_object_table_bit_test_full(
                        operand_offset,
                        (const volatile cb_u8 CB_NEAR *)source_cursor)) {
                    break;
                }
            } else if (source->kind == VM_C1_SOURCE_FLAG_KIND
                    && (((volatile bloodprg_vm_object_header CB_FAR *)
                        VM_C1_RECORD_AT(record_base, operand_offset))->flags
                        & VM_C1_OPERAND_SOURCE_FLAG) != 0u) {
                break;
            }
        }

        field_offset = (cb_u16)vm_field_offset(
            VM_C1_DESTINATION_SELECTOR, VM_C1_NAV_TARGET_KIND);
        record = (volatile bloodprg_vm_record_triple CB_FAR *)
            ((volatile cb_u8 CB_FAR *)target + field_offset);
    }

    if (record->kind != 0u) {
        return BLOODPRG_VM_CURSOR_AT(script_bytes, vm_branch_fail());
    }
    record->kind = VM_C1_RECORD_KIND;
    record->related = operand_offset;
    record->value = VM_C1_RECORD_VALUE;
    return script_bytes;
}

#undef VM_C1_RECORD_AT
