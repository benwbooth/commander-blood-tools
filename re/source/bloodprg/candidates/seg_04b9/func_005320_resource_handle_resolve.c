#include "../include/bloodprg_resource.h"

bloodprg_resource_resolve_result CB_FAR resource_handle_resolve(cb_u16 handle)
{
    bloodprg_resource_resolve_result result;
    const volatile bloodprg_resource_handle_entry *entry;

    result.segment = 0;
    result.offset = 0;
    result.loaded = 0;

    entry = &fs_resource_handle_table[handle];
    if (entry->unknown_02 & BLOODPRG_RESOURCE_FLAG_LOADED) {
        result.segment = entry->unknown_00;
        result.loaded = 1;
    }

    return result;
}
