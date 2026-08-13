#include "../include/bloodprg_resource.h"

void CB_FAR startup_resource_file_copy(
        volatile char CB_FAR *source_path,
        const volatile char CB_FAR *destination_path)
{
    volatile cb_u8 CB_FAR *buffer;
    cb_u32 remaining;
    cb_u16 source_handle;
    cb_u16 destination_handle;
    cb_u16 bytes_read;

    remaining = resource_name_lookup(source_path);
    if (remaining == 0) {
        return;
    }
    if (!cb_dos_open_read_only(source_path, &source_handle)) {
        return;
    }

    resource_copy_file_handle = source_handle;
    if (!cb_dos_create_truncate(destination_path, &destination_handle)) {
        return;
    }

    buffer = resource_copy_buffer;
    do {
        resource_copy_file_handle = destination_handle;
        bytes_read = cb_dos_read(source_handle, buffer, 0xfa00u);
        remaining -= (cb_u32)bytes_read;

        resource_copy_file_handle = source_handle;
        (void)cb_dos_write(destination_handle, buffer, bytes_read);
    } while (remaining != 0);

    cb_dos_close(destination_handle);
    cb_dos_close(source_handle);
}
