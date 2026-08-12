#include "../include/bloodprg_list.h"
#include "../include/bloodprg_resource.h"

int CB_NEAR list_d8c_activate_ready(void)
{
    volatile cb_u16 CB_FAR *entry;
    cb_u16 entry_extent;
    cb_u16 storage_segment;
    cb_u16 queued_bytes;

    if (list_d8c_active_segment != 0) {
        return 1;
    }

    queued_bytes = list_d8c_byte_count;
    if (queued_bytes == 0) {
        return 0;
    }

    entry = list_d8c_tail_pointer;
    entry_extent = *entry++;
    if (*entry != 0x6d6du && queued_bytes < entry_extent) {
        return 0;
    }

    storage_segment = list_d8c_default_entry_segment;
    if ((resource_flags & 0x0040u) != 0) {
        storage_segment = list_d8c_alternate_entry_segment;
    }
    list_d8c_activate_entry(entry_extent, entry, storage_segment);
    return 1;
}
