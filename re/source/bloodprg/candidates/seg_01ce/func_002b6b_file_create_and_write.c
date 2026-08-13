#include <dos.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_startup.h"

#define BLOODPRG_FILE_WRITE_BYTES 0x7d00u

cb_u32 CB_FAR file_create_and_write(
        const volatile char CB_FAR *path,
        const volatile cb_u8 CB_FAR *source,
        cb_u32 byte_count)
{
    cb_u16 file_handle;
    cb_u16 request_bytes;
    cb_u16 bytes_written;
    cb_u16 source_segment;
    cb_u16 source_offset;

    startup_write_directory_enter();
    snd_source_remaining = byte_count;

    if (!cb_dos_create_truncate(path, &file_handle)) {
        return 0;
    }
    resource_copy_file_handle = file_handle;

    do {
        request_bytes = (cb_u16)snd_source_remaining;
        if ((cb_u16)(snd_source_remaining >> 16) != 0) {
            request_bytes = BLOODPRG_FILE_WRITE_BYTES;
        }

        bytes_written = cb_dos_write(
                file_handle, source, request_bytes);
        snd_source_remaining -= (cb_u32)bytes_written;

        source_segment = (cb_u16)(FP_SEG(source) + (bytes_written >> 4));
        source_offset = (cb_u16)(FP_OFF(source) + (bytes_written & 0x0fu));
        source = (const volatile cb_u8 CB_FAR *)MK_FP(
                source_segment, source_offset);
    } while (snd_source_remaining != 0);

    cb_dos_close(file_handle);
    return byte_count;
}
