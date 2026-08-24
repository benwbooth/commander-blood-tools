#include <dos.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_byte_parser.h"
#include "../include/bloodprg_ems.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_startup.h"
#include "../include/bloodprg_vm.h"

void CB_FAR snd_stream_source_load(volatile char CB_NEAR *path)
{
    bloodprg_graphics_buffer_ptr volatile saved_framebuffer;
    cb_u32 seek_offset;
    cb_u16 source_handle;
    cb_u16 request_bytes;
    cb_u16 bytes_read;
    cb_u16 logical_page;
    cb_u16 remainder;
    cb_u16 index;
    cb_u8 character;
    volatile cb_u8 CB_FAR *page_frame;

    if ((voc_playback_enabled_gs & 1u) == 0
            || (snd_stream_channel_active & 1u) == 0) {
        return;
    }

    source_handle = resource_source_select(path);
    if ((resource_path_is_embedded & 1u) == 0) {
        snd_source_remaining = resource_name_lookup(path);
        resource_archive_offset = 0;
        (void)cb_dos_open_read_only(path, &source_handle);
    }

    snd_stream_page_count = 0;
    music_voc_name_changed = 0;
    snd_driver_pending_flag_gs = 1u;

    vm_text_reveal_cursor_gs = 0x0e2au;
    index = 0;
    do {
        character = (cb_u8)snd_wait_prompt_text[index];
        vm_text_buffer_gs[index] = (char)character;
        ++index;
    } while (character != 0);
    vm_subtitle_display_mode = 2u;
    vm_text_reveal_phase = 0;
    vm_presentation_hold_ready_gs = 0;
    saved_framebuffer = graphics_draw_framebuffer;
    graphics_draw_framebuffer = graphics_screen_buffer;
    subtitle_reveal_pump();
    graphics_draw_framebuffer = saved_framebuffer;
    vm_subtitle_display_mode = 0;
    vm_presentation_defer_a_gs = 0;

    seek_offset = resource_archive_offset;
    seek_offset = (seek_offset & 0xffff0000UL)
            | (cb_u16)((cb_u16)seek_offset + 0x001au);
    cb_dos_seek_absolute(source_handle, seek_offset);
    snd_source_remaining -= 0x1aUL;

    bytes_read = 0;
    if (snd_bank_ems_handle != -1) {
        page_frame = (volatile cb_u8 CB_FAR *)MK_FP(
                ems_page_frame_segment, 0u);
        snd_bank_storage_mode_gs = 0;
        snd_storage_cursor.ems.logical_page = 0;
        do {
            logical_page = snd_storage_cursor.ems.logical_page;
            cb_ems_map_page((cb_u16)snd_bank_ems_handle,
                    logical_page, 0);
            ++logical_page;
            cb_ems_map_page((cb_u16)snd_bank_ems_handle,
                    logical_page, 1u);
            ++logical_page;
            snd_storage_cursor.ems.logical_page = logical_page;

            request_bytes = snd_source_remaining > 0x8000UL
                    ? 0x8000u
                    : (cb_u16)snd_source_remaining;
            bytes_read = cb_dos_read(source_handle,
                    page_frame, request_bytes);
            snd_stream_page_count += 2u;
            snd_source_remaining -= bytes_read;
        } while (snd_source_remaining != 0);
    } else if (snd_bank_xms_handle != -1) {
        snd_bank_storage_mode_gs = 1u;
        snd_storage_cursor.xms_offset = 0;
        do {
            request_bytes = snd_source_remaining > 0x8000UL
                    ? 0x8000u
                    : (cb_u16)snd_source_remaining;
            bytes_read = cb_dos_read(source_handle,
                    snd_stream_storage, request_bytes);
            if (bytes_read == 0) {
                break;
            }

            xms_move_request.length =
                    (cb_u32)bytes_read + (bytes_read & 1u);
            xms_move_request.source_handle = 0;
            xms_move_request.source.pointer = snd_stream_storage;
            xms_move_request.destination_handle =
                    (cb_u16)snd_bank_xms_handle;
            xms_move_request.destination.offset =
                    snd_storage_cursor.xms_offset;
            snd_storage_cursor.xms_offset += 0x8000UL;
            cb_xms_move(&xms_move_request);

            snd_stream_page_count += 2u;
            snd_source_remaining -= bytes_read;
        } while (snd_source_remaining != 0);
    } else {
        snd_bank_storage_mode_gs = 2u;
        if (snd_bank_file_handle != 0) {
            cb_dos_close(snd_bank_file_handle);
        }
        startup_write_directory_enter();
        (void)cb_dos_create_game_file(
                snd_music_temp_filename, &snd_bank_file_handle);

        do {
            request_bytes = snd_source_remaining > 0x8000UL
                    ? 0x8000u
                    : (cb_u16)snd_source_remaining;
            bytes_read = cb_dos_read(source_handle,
                    snd_stream_storage, request_bytes);
            (void)cb_dos_write(snd_bank_file_handle,
                    snd_stream_storage, bytes_read);
            snd_stream_page_count += 2u;
            snd_source_remaining -= bytes_read;
        } while (snd_source_remaining != 0);
    }

    remainder = bytes_read & 0x3fffu;
    if (remainder == bytes_read) {
        --snd_stream_page_count;
    }
    snd_stream_final_page_bytes = remainder != 0 ? remainder : 0x4000u;

    if ((resource_path_is_embedded & 1u) == 0) {
        cb_dos_close(source_handle);
    }
}
