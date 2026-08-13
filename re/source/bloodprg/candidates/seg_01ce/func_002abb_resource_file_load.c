#include <dos.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_resource.h"

#define BLOODPRG_RESOURCE_READ_BYTES 0x7d00UL

cb_u32 CB_FAR resource_file_load(volatile char CB_FAR *path,
        volatile cb_u8 CB_FAR *destination)
{
    volatile bloodprg_dos_dta CB_FAR *dta;
    cb_u32 difference;
    cb_u16 file_handle;
    cb_u16 request_bytes;
    cb_u16 bytes_read;
    cb_u16 destination_segment;
    cb_u16 destination_offset;
    int embedded;

    file_handle = resource_source_select(path);
    embedded = (resource_path_is_embedded & 1u) != 0;
    if (!embedded) {
        dta = cb_dos_get_dta();
        (void)cb_dos_find_first(path);
        resource_archive_remaining = dta->file_size;
        snd_source_remaining = dta->file_size;
        if (!cb_dos_open_read_only(path, &file_handle)) {
            return 0;
        }
    }

    resource_copy_file_handle = file_handle;
    do {
        request_bytes = (cb_u16)BLOODPRG_RESOURCE_READ_BYTES;
        difference = snd_source_remaining - BLOODPRG_RESOURCE_READ_BYTES;
        if ((cb_i32)difference < 0) {
            request_bytes = (cb_u16)snd_source_remaining;
        }

        bytes_read = cb_dos_read(file_handle, destination, request_bytes);
        snd_source_remaining -= (cb_u32)bytes_read;

        destination_segment = (cb_u16)(FP_SEG(destination)
                + (bytes_read >> 4));
        destination_offset = (cb_u16)(FP_OFF(destination)
                + (bytes_read & 0x0fu));
        destination = (volatile cb_u8 CB_FAR *)MK_FP(
                destination_segment, destination_offset);
    } while (snd_source_remaining != 0);

    if (!embedded) {
        cb_dos_close(file_handle);
    }
    return resource_archive_remaining;
}
