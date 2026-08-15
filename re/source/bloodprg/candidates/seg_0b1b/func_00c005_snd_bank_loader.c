#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_ems.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_startup.h"

void CB_FAR snd_bank_loader(cb_u16 mode,
        volatile char CB_NEAR *path)
{
    volatile cb_u8 CB_FAR *staging;
    cb_u32 payload_remaining;
    cb_u32 clip_start;
    cb_u32 clip_end;
    cb_u16 source_handle;
    cb_u16 table_bytes;
    cb_u16 table_count;
    cb_u16 request_bytes;
    cb_u16 bytes_read;
    cb_u16 logical_page;
    cb_u16 index;

    if ((voc_playback_enabled_gs & 1u) == 0) {
        return;
    }

    source_handle = resource_source_select(path);
    if ((resource_path_is_embedded & 1u) == 0) {
        snd_source_remaining = resource_name_lookup(path);
        (void)cb_dos_open_read_only(path, &source_handle);
    }

    snd_source_remaining -= 4u;
    (void)cb_dos_read(source_handle,
            (volatile cb_u8 CB_FAR *)&snd_bank_header, 4u);
    table_count = (cb_u16)(snd_bank_header.clip_count + 1u);
    table_bytes = (cb_u16)(table_count * 4u);
    snd_source_remaining -= table_bytes;
    (void)cb_dos_read(source_handle,
            (volatile cb_u8 CB_FAR *)snd_source_offsets, table_bytes);

    if (mode == 0) {
        for (index = 0; index < snd_bank_header.clip_count; ++index) {
            clip_start = snd_source_offsets[index];
            clip_end = snd_source_offsets[index + 1u];
            snd_memory_clips[index].offset = (cb_u16)clip_start;
            snd_memory_clips[index].byte_count =
                    (cb_u16)((cb_u16)(clip_end - clip_start) - 1u);
        }
        (void)cb_dos_read(source_handle, snd_bank_memory,
                (cb_u16)snd_source_remaining);
    } else {
        payload_remaining = snd_source_remaining;
        snd_streamed_clip_count = snd_bank_header.clip_count;
        for (index = 0; index < table_count; ++index) {
            snd_streamed_offsets[index] = snd_source_offsets[index];
        }

        if (secondary_ems_handle != -1) {
            snd_storage_cursor.ems.logical_page = 0;
            do {
                logical_page = snd_storage_cursor.ems.logical_page;
                cb_ems_map_page((cb_u16)secondary_ems_handle,
                        logical_page, 0);
                ++logical_page;
                cb_ems_map_page((cb_u16)secondary_ems_handle,
                        logical_page, 1u);
                ++logical_page;
                snd_storage_cursor.ems.logical_page = logical_page;

                request_bytes = payload_remaining > 0x8000UL
                        ? 0x8000u
                        : (cb_u16)payload_remaining;
                bytes_read = cb_dos_read(source_handle,
                        ems_page_frame, request_bytes);
                payload_remaining -= bytes_read;
            } while (payload_remaining != 0);
        } else {
            staging = graphics_work_surface + 0x7d00u;
            if (secondary_xms_handle != -1) {
                snd_storage_cursor.xms_offset = 0;
                do {
                    request_bytes = payload_remaining > 0x7d00UL
                            ? 0x7d00u
                            : (cb_u16)payload_remaining;
                    bytes_read = cb_dos_read(source_handle,
                            staging, request_bytes);

                    xms_move_request.length =
                            (cb_u32)bytes_read + (bytes_read & 1u);
                    xms_move_request.source_handle = 0;
                    xms_move_request.source.pointer = staging;
                    xms_move_request.destination_handle =
                            (cb_u16)secondary_xms_handle;
                    xms_move_request.destination.offset =
                            snd_storage_cursor.xms_offset;
                    snd_storage_cursor.xms_offset += 0x7d00UL;
                    cb_xms_move(&xms_move_request);
                    payload_remaining -= bytes_read;
                } while (payload_remaining != 0);
            } else {
                if (snd_voice_file_handle != 0) {
                    cb_dos_close(snd_voice_file_handle);
                }
                startup_write_directory_enter();
                (void)cb_dos_create_game_file(
                        snd_voice_temp_filename, &snd_voice_file_handle);

                do {
                    request_bytes = payload_remaining > 0x7d00UL
                            ? 0x7d00u
                            : (cb_u16)payload_remaining;
                    bytes_read = cb_dos_read(source_handle,
                            staging, request_bytes);
                    (void)cb_dos_write(snd_voice_file_handle,
                            staging, bytes_read);
                    payload_remaining -= bytes_read;
                } while (payload_remaining != 0);
            }
        }
    }

    if ((resource_path_is_embedded & 1u) == 0) {
        cb_dos_close(source_handle);
    }
}
