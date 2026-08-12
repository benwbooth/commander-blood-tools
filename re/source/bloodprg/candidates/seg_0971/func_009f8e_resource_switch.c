#include "../include/bloodprg_list.h"
#include "../include/bloodprg_resource.h"

int CB_NEAR resource_switch(cb_u16 resource_id)
{
    bloodprg_resource_descriptor *descriptor;
    volatile bloodprg_dos_dta CB_FAR *dta;
    volatile cb_u8 CB_FAR *stream;
    cb_u16 saved_byte_count;
    cb_u16 saved_head_offset;
    cb_u16 entry_extent;
    cb_u16 cursor_offset;
    cb_u16 end_offset;
    cb_u16 byte_count;
    cb_u16 handle;
    cb_u16 table_offset;
    cb_u32 relative_offset;
    int read_succeeded;

    resource_requested_id = resource_id;
    close_file_d5b();
    list_d8c_init();
    list_d8c_state_byte = 0;
    list_d8c_bounds_init();

    resource_active_id = resource_id;
    descriptor = lookup_table_1fb5(resource_id);
    descriptor->variant = resource_variant;
    resource_flags = (cb_u16)descriptor->flags
            | ((cb_u16)descriptor->variant << 8);

    resource_source_remaining = resource_archive_size;
    handle = 0;
    resource_source_offset = 0;

    if ((resource_source_is_banked & 1u) == 0) {
        handle = path_builder_gs_relative(descriptor->filename);
        resource_source_remaining = resource_archive_remaining;
        resource_source_offset = resource_archive_offset;

        if ((resource_path_is_embedded & 1u) == 0) {
            dta = cb_dos_get_dta();
            cb_dos_find_first(resource_path_buffer);
            resource_source_remaining = dta->file_size;
            if (!cb_dos_open_read_only(resource_path_buffer, &handle)) {
                list_d8c_file_handle = handle;
                return 0;
            }
            resource_source_offset = 0;
        }
    }
    list_d8c_file_handle = handle;

    saved_byte_count = list_d8c_byte_count;
    saved_head_offset = list_d8c_head_offset;
    read_succeeded = list_d8c_read(&entry_extent, &cursor_offset);
    if (read_succeeded) {
        list_d8c_entry_metric = entry_extent;
        end_offset = (cb_u16)(cursor_offset + entry_extent);
        if (end_offset < cursor_offset
                || end_offset > list_d8c_buffer_end_offset) {
            list_d8c_head_offset = 0;
        }

        byte_count = (cb_u16)(entry_extent - 2u);
        read_succeeded = ems_paged_read(byte_count);
    }
    list_d8c_head_offset = saved_head_offset;
    list_d8c_byte_count = saved_byte_count;
    if (!read_succeeded) {
        return 0;
    }

    stream = list_d8c_buffer + list_d8c_head_offset;
    entry_extent = *(volatile cb_u16 CB_FAR *)stream;
    stream += 2;
    cursor_offset = (cb_u16)(list_d8c_head_offset + 2u);
    end_offset = (cb_u16)(cursor_offset + entry_extent);
    if (end_offset < cursor_offset
            || end_offset > list_d8c_buffer_end_offset) {
        stream = list_d8c_buffer;
    }

    resource_ready_marker = 0xffu;
    stream = resource_palette_blocks_apply(stream);
    while (*stream == 0xffu) {
        ++stream;
    }

    table_offset = (resource_flags & 0x0004u) != 0 ? 0x10u : 0;
    relative_offset = *(volatile cb_u32 CB_FAR *)(stream + table_offset);
    resource_range_start = resource_source_offset + relative_offset;
    resource_range_remaining = resource_source_remaining - relative_offset;

    table_offset = (cb_u16)(list_d8c_entry_metric * 4u);
    relative_offset = *(volatile cb_u32 CB_FAR *)(stream + table_offset);
    resource_index_start = resource_source_offset + relative_offset;
    resource_index_remaining = resource_source_remaining - relative_offset;

    return 1;
}
