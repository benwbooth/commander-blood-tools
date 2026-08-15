#include <dos.h>

#include "../include/bloodprg_ems.h"
#include "../include/bloodprg_list.h"
#include "../include/bloodprg_resource.h"

int CB_NEAR ems_paged_read(cb_u16 byte_count)
{
    cb_u32 source_offset;
    cb_u16 logical_page;
    cb_u16 transferred;
    cb_u16 handle;
    cb_u8 physical_page;
    volatile cb_u8 CB_FAR *destination;

    if ((resource_source_is_banked & 1u) != 0
            && resource_ems_handle != -1) {
        source_offset = resource_source_offset;
        logical_page = (cb_u16)(source_offset >> 14);
        handle = (cb_u16)resource_ems_handle;
        for (physical_page = 0; physical_page < 4u; ++physical_page) {
            cb_ems_map_page(handle, logical_page, physical_page);
            ++logical_page;
        }
        destination = (volatile cb_u8 CB_FAR *)MK_FP(
                list_d8c_head_segment, list_d8c_head_offset);
        far_memmove(
                destination,
                (const volatile cb_u8 CB_FAR *)MK_FP(
                        ems_page_frame_segment,
                        (cb_u16)(source_offset & 0x3fffu)),
                byte_count);
        transferred = byte_count;
    } else if ((resource_source_is_banked & 1u) != 0
            && resource_xms_handle != -1) {
        xms_move_request.length =
                (cb_u32)byte_count + (byte_count & 1u);
        xms_move_request.source_handle =
                (cb_u16)resource_xms_handle;
        xms_move_request.source.offset = resource_source_offset;
        xms_move_request.destination_handle = 0;
        xms_move_request.destination.pointer = (volatile cb_u8 CB_FAR *)MK_FP(
                list_d8c_head_segment, list_d8c_head_offset);
        cb_xms_move(&xms_move_request);
        transferred = byte_count;
    } else {
        handle = list_d8c_file_handle;
        if (handle < 1u) {
            return 0;
        }
        do {
            cb_dos_seek_absolute(handle, resource_source_offset);
            destination = (volatile cb_u8 CB_FAR *)MK_FP(
                    list_d8c_head_segment, list_d8c_head_offset);
            transferred = cb_dos_read(
                    handle,
                    destination,
                    byte_count);
        } while (transferred < byte_count);
    }

    resource_source_remaining -= transferred;
    resource_source_offset += transferred;
    list_d8c_head_offset += transferred;
    list_d8c_byte_count += transferred;
    return 1;
}
