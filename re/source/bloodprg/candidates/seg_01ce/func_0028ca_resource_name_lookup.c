#include "../include/bloodprg_resource.h"

cb_u32 CB_FAR resource_name_lookup(volatile char CB_FAR *filename)
{
    volatile bloodprg_dos_dta CB_FAR *dta;
    cb_u32 byte_count;

    (void)resource_source_select(filename);
    byte_count = resource_archive_remaining;
    if ((resource_path_is_embedded & 1u) == 0) {
        dta = cb_dos_get_dta();
        (void)cb_dos_find_first(filename);
        byte_count = dta->file_size;
    }

    return byte_count;
}
