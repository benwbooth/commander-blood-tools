#include <dos.h>

#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_ems.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_resource.h"

#define BLOODPRG_ARCHIVE_INDEX_STAGING_OFFSET 0x7d00u
#define BLOODPRG_ARCHIVE_INDEX_XMS_BYTES 0x7d00UL

cb_u16 CB_NEAR resource_archive_match(volatile char CB_NEAR *filename)
{
    volatile bloodprg_resource_archive_entry CB_FAR *entry;
    volatile cb_u8 CB_FAR *archive_index;
    volatile cb_u8 CB_NEAR *character;
    cb_u32 payload_offset;
    cb_u16 archive_handle;
    cb_u16 character_index;
    cb_u8 archive_character;
    cb_u8 filename_character;
    cb_u8 physical_page;

    archive_handle = resource_archive_handle;
    if (archive_handle == 0) {
        return 0;
    }

    if (small_ems_handle != -1) {
        for (physical_page = 0; physical_page < 4u; ++physical_page) {
            cb_ems_map_page((cb_u16)small_ems_handle,
                    physical_page, physical_page);
        }
        archive_index = (volatile cb_u8 CB_FAR *)MK_FP(
                ems_page_frame_segment, 0u);
    } else {
        archive_index = graphics_work_surface
                + BLOODPRG_ARCHIVE_INDEX_STAGING_OFFSET;
        if (small_xms_handle != -1) {
            xms_move_request.length = BLOODPRG_ARCHIVE_INDEX_XMS_BYTES;
            xms_move_request.source_handle = (cb_u16)small_xms_handle;
            xms_move_request.source.offset = 0;
            xms_move_request.destination_handle = 0;
            xms_move_request.destination.pointer = archive_index;
            cb_xms_move(&xms_move_request);
        } else {
            (void)cb_dos_read(resource_archive_cache_handle,
                    archive_index, 0xffffu);
            archive_index = (volatile cb_u8 CB_FAR *)MK_FP(
                    FP_SEG(graphics_work_surface),
                    BLOODPRG_ARCHIVE_INDEX_STAGING_OFFSET);
        }
    }

    character = (volatile cb_u8 CB_NEAR *)filename;
    do {
        filename_character = *character;
        if (filename_character >= (cb_u8)'a') {
            filename_character &= 0xdfu;
            *character = filename_character;
        }
        ++character;
    } while (filename_character != 0);

    entry = (volatile bloodprg_resource_archive_entry CB_FAR *)
            (archive_index + 2u);
    while (entry->filename[0] != '\0') {
        character_index = 0;
        do {
            archive_character =
                    ((volatile cb_u8 CB_FAR *)entry->filename)[character_index];
            filename_character =
                    ((volatile cb_u8 CB_NEAR *)filename)[character_index];
            if (archive_character != filename_character) {
                break;
            }
            ++character_index;
        } while (archive_character != 0);

        if (archive_character == 0 && filename_character == 0) {
            resource_path_is_embedded = 1;
            resource_archive_remaining = entry->byte_count;
            snd_source_remaining = entry->byte_count;
            payload_offset = entry->file_offset;
            resource_archive_offset = payload_offset;
            cb_dos_seek_absolute(archive_handle, payload_offset);
            return archive_handle;
        }
        ++entry;
    }

    return 0;
}
