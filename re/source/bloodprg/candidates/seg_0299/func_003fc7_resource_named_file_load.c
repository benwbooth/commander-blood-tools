#include <dos.h>

#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_resource.h"

int CB_FAR resource_named_file_load(cb_u16 resource_id,
        volatile cb_u8 CB_FAR *direct_destination)
{
    bloodprg_resource_allocation_result allocation;
    volatile bloodprg_dos_dta CB_FAR *dta;
    volatile char *filename;
    volatile cb_u8 CB_FAR *destination;
    cb_u32 remaining_bytes;
    cb_u16 file_handle;
    cb_u16 file_header;
    cb_u16 bytes_read;

    filename = resource_name_table[resource_id].filename;
    (void)resource_source_select(filename);

    dta = cb_dos_get_dta();
    if (!cb_dos_find_first(filename)) {
        return -1;
    }
    remaining_bytes = dta->file_size;
    if (!cb_dos_open_read_only(filename, &file_handle)) {
        return -1;
    }

    (void)cb_dos_read(file_handle,
            (volatile cb_u8 CB_FAR *)&resource_file_header, 2u);
    file_header = resource_file_header;
    if (file_header & 2u) {
        resource_palette_file_blocks_apply(file_handle,
                &resource_file_header, &remaining_bytes);
    }

    destination = direct_destination;
    if ((cb_i16)resource_id >= 0) {
        allocation = resource_allocate(resource_id, remaining_bytes);
        if (allocation.status < 0) {
            cb_dos_close(file_handle);
            return -1;
        }
        if (allocation.status != 0) {
            cb_dos_close(file_handle);
            return 0;
        }
        destination = allocation.destination;
    }

    *(volatile cb_u16 CB_FAR *)destination = file_header;
    destination += 2u;
    remaining_bytes -= 2u;
    for (;;) {
        bytes_read = cb_dos_read(file_handle, destination, 0x7d00u);
        remaining_bytes -= bytes_read;
        if (remaining_bytes == 0) {
            break;
        }
        destination = (volatile cb_u8 CB_FAR *)MK_FP(
                (cb_u16)(FP_SEG(destination) + (bytes_read >> 4)),
                (cb_u16)(FP_OFF(destination) + (bytes_read & 0x000fu)));
    }

    cb_dos_close(file_handle);
    return 0;
}
