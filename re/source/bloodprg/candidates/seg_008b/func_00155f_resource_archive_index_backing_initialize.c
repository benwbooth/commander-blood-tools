#include <dos.h>

#include "../include/bloodprg_ems.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_startup.h"

#define ARCHIVE_INDEX_READ_BYTES 0xffffu
#define ARCHIVE_INDEX_BACKING_BYTES 0x10000UL
#define ARCHIVE_INDEX_DWORD_COUNT 0x4000u
#define ARCHIVE_INDEX_EMS_PAGES 4u
#define ARCHIVE_INDEX_FILE_SOURCE_OFFSET 0x00cbu

void CB_NEAR resource_archive_index_backing_initialize(void)
{
    volatile cb_u32 CB_FAR *source;
    volatile cb_u32 CB_FAR *destination;
    cb_u16 file_handle;
    cb_u16 work_segment;
    cb_u16 dword_count;
    cb_u8 page;

    startup_original_directory_restore();
    if (!cb_dos_open_read_only(
            (const volatile char CB_FAR *)resource_archive_filename,
            &file_handle)) {
        return;
    }

    resource_archive_handle = file_handle;
    (void)cb_dos_read(
            file_handle, graphics_work_surface, ARCHIVE_INDEX_READ_BYTES);
    work_segment = FP_SEG(graphics_work_surface);

    if (small_ems_handle != -1) {
        page = 0;
        do {
            cb_ems_map_page((cb_u16)small_ems_handle, page, page);
            ++page;
        } while (page != ARCHIVE_INDEX_EMS_PAGES);

        source = (volatile cb_u32 CB_FAR *)MK_FP(work_segment, 0u);
        destination = (volatile cb_u32 CB_FAR *)MK_FP(
                ems_page_frame_segment, 0u);
        dword_count = ARCHIVE_INDEX_DWORD_COUNT;
        do {
            *destination++ = *source++;
            --dword_count;
        } while (dword_count != 0u);
        return;
    }

    if (small_xms_handle != -1) {
        xms_move_request.length = ARCHIVE_INDEX_BACKING_BYTES;
        xms_move_request.source_handle = 0;
        xms_move_request.source.pointer =
                (volatile cb_u8 CB_FAR *)MK_FP(work_segment, 0u);
        xms_move_request.destination_handle = (cb_u16)small_xms_handle;
        xms_move_request.destination.offset = 0;
        cb_xms_move(&xms_move_request);
        return;
    }

    startup_write_directory_enter();
    (void)cb_dos_create_game_file(
            resource_archive_cache_filename,
            &resource_archive_cache_handle);
    (void)cb_dos_write(
            resource_archive_cache_handle,
            (const volatile cb_u8 CB_FAR *)MK_FP(
                    work_segment, ARCHIVE_INDEX_FILE_SOURCE_OFFSET),
            ARCHIVE_INDEX_READ_BYTES);
}
