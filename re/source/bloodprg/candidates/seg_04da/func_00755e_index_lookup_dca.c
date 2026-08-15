#include "../include/bloodprg_byte_parser.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_startup.h"

#define BACKGROUND_NAME_OFFSET 3u

const cb_u8 CB_FAR *CB_NEAR index_lookup_dca(
    const cb_u8 CB_FAR *script_bytes)
{
    const cb_u8 CB_FAR *cursor;
    cb_u16 slot_index;
    cb_u16 name_index;
    cb_u16 compare_index;
    cb_u16 source_handle;
    cb_u16 output_handle;
    cb_u16 bytes_read;
    cb_u8 value;

    cursor = script_bytes;
    slot_index = (cb_u16)(cb_i16)(cb_i8)(cb_u8)(*cursor++ - 1u);

    name_index = 0u;
    for (;;) {
        value = *cursor;
        if ((cb_i8)value < 0 || value < 0x20u) {
            break;
        }
        ++cursor;
        byte_parser_background_path[
                BACKGROUND_NAME_OFFSET + name_index++] = (char)value;
    }
    byte_parser_background_path[
            BACKGROUND_NAME_OFFSET + name_index] = '\0';

    compare_index = 0u;
    while (byte_parser_background_path[
            BACKGROUND_NAME_OFFSET + compare_index] != '\0') {
        if (byte_parser_background_path[
                BACKGROUND_NAME_OFFSET + compare_index]
                != byte_parser_background_slots[slot_index][compare_index]) {
            goto cache_miss;
        }
        ++compare_index;
    }
    return cursor;

cache_miss:
    startup_write_directory_enter();
    (void)cb_dos_delete(byte_parser_background_slots[slot_index]);

    name_index = 0u;
    do {
        value = (cb_u8)byte_parser_background_path[
                BACKGROUND_NAME_OFFSET + name_index];
        byte_parser_background_slots[slot_index][name_index++] = (char)value;
    } while (value != 0u);

    (void)cb_dos_create_truncate(
            byte_parser_background_slots[slot_index], &output_handle);
    source_handle = resource_source_select(byte_parser_background_path);
    if ((resource_path_is_embedded & 1u) == 0u) {
        byte_parser_background_source_size =
                resource_name_lookup(byte_parser_background_path);
        (void)cb_dos_open_read_only(
                byte_parser_background_path, &source_handle);
    }

    bytes_read = cb_dos_read(source_handle, byte_parser_back_buffer,
            (cb_u16)byte_parser_background_source_size);
    (void)cb_dos_write(output_handle, byte_parser_back_buffer, bytes_read);
    if ((resource_path_is_embedded & 1u) == 0u) {
        cb_dos_close(source_handle);
    }
    cb_dos_close(output_handle);
    return cursor;
}
