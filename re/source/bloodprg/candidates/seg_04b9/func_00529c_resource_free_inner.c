#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_ems.h"

void CB_FAR resource_free_inner(cb_u16 handle)
{
    volatile bloodprg_resource_handle_entry *released;
    volatile bloodprg_resource_handle_entry *shifted_entry;
    volatile cb_u8 CB_FAR *destination;
    const volatile cb_u8 CB_FAR *source;
    cb_u32 released_size;
    cb_u32 moved_bytes;
    cb_u16 released_segment;
    cb_u16 paragraphs;
    cb_u16 position;
    cb_u16 shifted_handle;

    released = &fs_resource_handle_table[handle];
    released_segment = released->unknown_00;
    released->unknown_02 &= (cb_u16)~BLOODPRG_RESOURCE_FLAG_LOADED;
    released_size = released->field_04;
    resource_free_bytes += released_size;
    paragraphs = (cb_u16)(released_size >> 4);
    resource_pool_end_segment =
            (cb_u16)(resource_pool_end_segment - paragraphs);

    position = 0u;
    while (resource_resident_handles[position] != handle) {
        ++position;
    }

    moved_bytes = 0u;
    do {
        shifted_handle = resource_resident_handles[position + 1u];
        resource_resident_handles[position] = shifted_handle;
        if ((cb_i16)shifted_handle < 0) {
            break;
        }

        shifted_entry = &fs_resource_handle_table[shifted_handle];
        shifted_entry->unknown_00 =
                (cb_u16)(shifted_entry->unknown_00 - paragraphs);
        moved_bytes += shifted_entry->field_04;
        ++position;
    } while (1);

    if (moved_bytes != 0u) {
        destination = (volatile cb_u8 CB_FAR *)
                ((cb_u32)released_segment << 16);
        source = (const volatile cb_u8 CB_FAR *)
                ((cb_u32)(cb_u16)(released_segment + paragraphs) << 16);
        far_memmove(destination, source, moved_bytes);
    }
}
