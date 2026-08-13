#include "../include/bloodprg_resource.h"

int CB_FAR resource_load_by_id(cb_u16 resource_id)
{
    bloodprg_resource_allocation_result allocation;
    const volatile char CB_FS_DATA *resource_name;
    const volatile char CB_FAR *filename;
    cb_u32 byte_count;

    resource_name = resource_name_table[resource_id].filename;
    filename = resource_name;
    byte_count = resource_name_lookup(filename);
    if (byte_count == 0) {
        return 0;
    }

    allocation = resource_allocate(resource_id, byte_count);
    if (allocation.status < 0) {
        return 0;
    }
    if (allocation.status != 0) {
        return 1;
    }

    return resource_file_load(filename, allocation.destination) != 0;
}
