#include <dos.h>

#include "../include/bloodprg_resource.h"

#define BLOODPRG_RESOURCE_FLAG_LOCKED 0x0002u
#define BLOODPRG_RESOURCE_FLAG_SPECIAL 0x000cu

bloodprg_resource_allocation_result CB_FAR resource_allocate(
        cb_u16 handle, cb_u32 byte_count)
{
    bloodprg_resource_allocation_result result;
    volatile bloodprg_resource_handle_entry *entry;
    volatile bloodprg_resource_handle_entry *candidate_entry;
    volatile bloodprg_resource_handle_entry CB_FAR *special_entry;
    cb_u32 rounded_byte_count;
    cb_u32 byte_deficit;
    cb_u16 resident_count;
    cb_u16 candidate_count;
    cb_u16 candidate_handle;

    result.status = -1;
    result.destination = (volatile cb_u8 CB_FAR *)0;
    resource_current_handle = handle;
    resource_current_entry_offset = (cb_u16)(handle * 8u);
    entry = &fs_resource_handle_table[handle];

    if (entry->unknown_02 & BLOODPRG_RESOURCE_FLAG_LOADED) {
        entry->unknown_02 |= BLOODPRG_RESOURCE_FLAG_LOCKED;
        result.status = 1;
        result.destination = (volatile cb_u8 CB_FAR *)MK_FP(
                entry->unknown_00, 0u);
        return result;
    }

    rounded_byte_count = (byte_count + 15u) & 0xfffffff0UL;
    if ((cb_i32)rounded_byte_count > (cb_i32)resource_free_bytes_gs) {
        resident_count = 0u;
        while (resident_count < 256u &&
                resource_resident_handles[resident_count] != 0xffffu) {
            ++resident_count;
        }

        byte_deficit = rounded_byte_count - resource_free_bytes_gs;
        candidate_count = 0u;
        while (resident_count != 0u) {
            --resident_count;
            candidate_handle = resource_resident_handles[resident_count];
            candidate_entry = &fs_resource_handle_table[candidate_handle];
            if ((candidate_entry->unknown_02 &
                    BLOODPRG_RESOURCE_FLAG_LOCKED) == 0) {
                resource_eviction_handles[candidate_count++] = candidate_handle;
                byte_deficit -= candidate_entry->field_04;
                if ((cb_i32)byte_deficit <= 0) {
                    break;
                }
            }
        }
        resource_eviction_handles[candidate_count] = 0xffffu;

        if ((cb_i32)byte_deficit >= 0) {
            cb_resource_allocation_failure(2u);
            return result;
        }

        candidate_count = 0u;
        while ((cb_i16)resource_eviction_handles[candidate_count] >= 0) {
            resource_free_inner(resource_eviction_handles[candidate_count]);
            ++candidate_count;
        }
    }

    entry = &fs_resource_handle_table[handle];
    entry->unknown_00 = resource_pool_end_segment_gs;
    entry->unknown_02 |= BLOODPRG_RESOURCE_FLAG_LOADED;
    entry->field_04 = rounded_byte_count;
    resource_free_bytes_gs -= rounded_byte_count;
    resource_pool_end_segment_gs = (cb_u16)(resource_pool_end_segment_gs +
            (cb_u16)(rounded_byte_count >> 4));

    resident_count = 0u;
    while (resident_count < 256u &&
            resource_resident_handles[resident_count] != 0xffffu) {
        ++resident_count;
    }
    resource_resident_handles[resident_count] = handle;
    resource_resident_handles[resident_count + 1u] = 0xffffu;

    result.destination = (volatile cb_u8 CB_FAR *)MK_FP(
            entry->unknown_00, 0u);
    if (entry->unknown_02 & BLOODPRG_RESOURCE_FLAG_SPECIAL) {
        special_entry =
                (volatile bloodprg_resource_handle_entry CB_FAR *)MK_FP(
                        entry->unknown_00, (cb_u16)(handle * 8u));
        special_entry->unknown_02 |= BLOODPRG_RESOURCE_FLAG_LOCKED;
        result.destination = (volatile cb_u8 CB_FAR *)MK_FP(
                special_entry->unknown_00, 0u);
        result.status = 1;
    } else {
        result.status = 0;
    }
    return result;
}
