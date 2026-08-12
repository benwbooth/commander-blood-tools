#include "../include/bloodprg_resource.h"

void CB_FAR resource_release(cb_u16 handle)
{
    const volatile bloodprg_resource_handle_entry *entry;

    entry = &fs_resource_handle_table[handle];
    if (entry->unknown_02 & BLOODPRG_RESOURCE_FLAG_LOADED) {
        resource_free_inner(handle);
    }
}
