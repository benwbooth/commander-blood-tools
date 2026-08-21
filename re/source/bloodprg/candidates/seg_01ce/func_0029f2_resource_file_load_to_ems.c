#include <dos.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_ems.h"
#include "../include/bloodprg_resource.h"

#define BLOODPRG_EMS_READ_BYTES 0x8000UL

void CB_FAR resource_file_load_to_ems(volatile char CB_FAR *path)
{
    volatile bloodprg_dos_dta CB_FAR *dta;
    cb_u32 difference;
    cb_u32 file_size;
    cb_u16 file_handle;
    cb_u16 logical_page;
    cb_u16 request_bytes;
    cb_u16 bytes_read;
    cb_u8 physical_page;
    volatile cb_u8 CB_FAR *page_frame;

    file_handle = resource_source_select(path);
    if ((resource_path_is_embedded & 1u) == 0) {
        dta = cb_dos_get_dta();
        (void)cb_dos_find_first(path);
        file_size = dta->file_size;
        resource_archive_remaining = file_size;
        snd_source_remaining = file_size;
        if (!cb_dos_open_read_only(path, &file_handle)) {
            return;
        }
    }

    resource_copy_file_handle = file_handle;
    page_frame = (volatile cb_u8 CB_FAR *)MK_FP(
            ems_page_frame_segment, 0u);
    snd_storage_cursor.xms_offset = 0;
    do {
        logical_page = snd_storage_cursor.ems.logical_page;
        physical_page = 0;
        do {
            cb_ems_map_page((cb_u16)resource_ems_handle,
                    logical_page, physical_page);
            ++logical_page;
            ++physical_page;
        } while (physical_page != 2u);
        snd_storage_cursor.ems.logical_page = logical_page;

        request_bytes = (cb_u16)BLOODPRG_EMS_READ_BYTES;
        difference = snd_source_remaining - BLOODPRG_EMS_READ_BYTES;
        if ((cb_i32)difference < 0) {
            request_bytes = (cb_u16)snd_source_remaining;
        }

        bytes_read = cb_dos_read(
                file_handle, page_frame, request_bytes);
        snd_source_remaining -= (cb_u32)bytes_read;
    } while (snd_source_remaining != 0);

    if ((resource_path_is_embedded & 1u) == 0) {
        cb_dos_close(file_handle);
    }
    resource_archive_size = resource_archive_remaining;
}
