#include "../include/bloodprg_list.h"
#include "../include/bloodprg_resource.h"

void CB_NEAR close_file_d5b(void)
{
    cb_u16 handle = list_d8c_file_handle;

    if (handle != 0 && handle != resource_archive_handle) {
        list_d8c_file_handle = 0;
        cb_dos_close(handle);
        list_d8c_bounds_init();
    }
}
