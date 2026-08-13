#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_ems.h"
#include "../include/bloodprg_resource.h"

#define BLOODPRG_XMS_READ_BYTES 0x7d00UL

void CB_FAR resource_file_load_to_xms(volatile char CB_FAR *path,
        volatile cb_u8 CB_FAR *staging_buffer)
{
    volatile bloodprg_dos_dta CB_FAR *dta;
    cb_u32 difference;
    cb_u32 file_size;
    cb_u16 file_handle;
    cb_u16 request_bytes;
    cb_u16 bytes_read;
    cb_u16 move_bytes;

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
    snd_storage_cursor.xms_offset = 0;
    do {
        request_bytes = (cb_u16)BLOODPRG_XMS_READ_BYTES;
        difference = snd_source_remaining - BLOODPRG_XMS_READ_BYTES;
        if ((cb_i32)difference < 0) {
            request_bytes = (cb_u16)snd_source_remaining;
        }

        bytes_read = cb_dos_read(
                file_handle, staging_buffer, request_bytes);
        snd_source_remaining -= (cb_u32)bytes_read;

        move_bytes = bytes_read;
        if ((bytes_read & 1u) != 0) {
            ++move_bytes;
        }
        xms_move_request.length = (cb_u32)move_bytes;
        xms_move_request.source_handle = 0;
        xms_move_request.source.pointer = staging_buffer;
        xms_move_request.destination_handle = (cb_u16)resource_xms_handle;
        xms_move_request.destination.offset = snd_storage_cursor.xms_offset;
        snd_storage_cursor.xms_offset += BLOODPRG_XMS_READ_BYTES;
        cb_xms_move(&xms_move_request);
    } while (snd_source_remaining != 0);

    if ((resource_path_is_embedded & 1u) == 0) {
        cb_dos_close(file_handle);
    }
    resource_archive_size = resource_archive_remaining;
}
