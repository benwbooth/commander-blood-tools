#include <dos.h>

#include "../include/bloodprg_byte_parser.h"
#include "../include/bloodprg_graphics.h"
#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_vm.h"

#define DESCRIPT_DIRECTORY_ENTRY_BYTES 18u
#define BYTE_PARSER_DETAIL_BASE 0x2154u
#define BYTE_PARSER_TABLE_BASE 0x1320u
#define BYTE_PARSER_STREAM_BASE 0x0F1Au
#define BYTE_PARSER_ASSET_BASE 0x1FDBu

#pragma pack(1)
typedef struct bloodprg_descript_directory_entry {
    cb_u8 name[16];
    cb_u16 record_offset;
} bloodprg_descript_directory_entry;
#pragma pack()

int CB_FAR vm_c2_descript_lookup(
    const volatile cb_u8 CB_FAR *record_name)
{
    volatile bloodprg_descript_directory_entry CB_FAR *directory;
    const volatile cb_u8 CB_FAR *directory_name;
    const volatile cb_u8 CB_FAR *wanted_name;
    const cb_u8 CB_FAR *script_bytes;
    cb_u16 file_handle;
    cb_u16 directory_bytes;
    cb_u8 directory_character;
    cb_u8 wanted_character;
    cb_u8 opcode;
    int found;
    int result;

    fs_name_area_dirty = 0u;
    music_voc_name_changed = 0u;
    byte_parser_table_131e_count = 0u;
    byte_parser_detail_cursor = (cb_game_char_ptr)BYTE_PARSER_DETAIL_BASE;
    byte_parser_table_131a_cursor = (cb_game_char_ptr)(
            BYTE_PARSER_TABLE_BASE
            + ((cb_u16)(cb_u8)vm_record_string_slots[0][0] << 7));
    byte_parser_stream_0f18_cursor =
            (cb_game_char_ptr)BYTE_PARSER_STREAM_BASE;
    byte_parser_asset_cursor = (cb_game_word_ptr)BYTE_PARSER_ASSET_BASE;
    byte_parser_b16_flag = 0u;

    (void)resource_source_select(descript_database_path);
    if (!cb_dos_open_read_only(descript_database_path, &file_handle)) {
        return 0;
    }

    (void)cb_dos_read(file_handle,
            (volatile cb_u8 CB_FAR *)&descript_directory_count,
            sizeof(descript_directory_count));
    directory_bytes = (cb_u16)(
            descript_directory_count * DESCRIPT_DIRECTORY_ENTRY_BYTES);
    (void)cb_dos_read(file_handle, graphics_work_surface, directory_bytes);

    directory = (volatile bloodprg_descript_directory_entry CB_FAR *)
            graphics_work_surface;
    found = 0;
    for (;;) {
        directory_name = directory->name;
        wanted_name = record_name;
        for (;;) {
            directory_character = *directory_name++;
            wanted_character = *wanted_name++;
            if (directory_character == 0u && wanted_character == 0u) {
                found = 1;
                break;
            }
            if (directory_character != wanted_character) {
                break;
            }
        }
        if (found) {
            break;
        }
        ++directory;
        --descript_directory_count;
        if (descript_directory_count == 0u) {
            break;
        }
    }

    result = 0;
    if (found) {
        cb_dos_seek_absolute(file_handle, (cb_u32)directory->record_offset);
        (void)cb_dos_read(file_handle,
                (volatile cb_u8 CB_FAR *)&descript_record_length,
                sizeof(descript_record_length));
        (void)cb_dos_read(file_handle, graphics_work_surface,
                (cb_u16)(descript_record_length - 2u));

        script_bytes = (const cb_u8 CB_FAR *)graphics_work_surface;
        for (;;) {
            opcode = *script_bytes++;
            if ((cb_i8)(opcode - 1u) < 0) {
                break;
            }
            switch (opcode) {
            case 0x01u:
                byte_parser_op_01_mark_b16();
                break;
            case 0x02u:
                byte_parser_op_02_mark_b16();
                break;
            case 0x03u:
                script_bytes = index_lookup_dca(script_bytes);
                break;
            case 0x04u:
                byte_parser_op_04_mark_b16();
                break;
            case 0x05u:
                script_bytes = credit_presenter_b_cryo(script_bytes);
                break;
            case 0x06u:
                script_bytes = byte_parser_copy_20b8_printable(script_bytes);
                break;
            case 0x07u:
                script_bytes = dlg_line_asset_table_fill(script_bytes);
                break;
            case 0x08u:
                script_bytes = (const cb_u8 CB_FAR *)
                        byte_parser_store_word_1fa5(
                            (const cb_u16 CB_FAR *)script_bytes);
                break;
            case 0x09u:
                script_bytes = byte_parser_copy_2460_printable(script_bytes);
                break;
            case 0x0Au:
                script_bytes = byte_parser_copy_247a_printable(script_bytes);
                break;
            case 0x0Bu:
                script_bytes = index_lookup_1fd7(script_bytes);
                break;
            case 0x0Cu:
                script_bytes = byte_parser_copy_131a_entry(script_bytes);
                break;
            case 0x0Du:
                script_bytes = byte_parser_stream_0f18_append(script_bytes);
                break;
            case 0x0Eu:
                script_bytes = fs_name_area_read(script_bytes);
                break;
            case 0x0Fu:
                byte_parser_op_0f_mark_b16();
                break;
            case 0x10u:
                script_bytes = byte_parser_copy_24c6_printable(script_bytes);
                break;
            case 0x11u:
                script_bytes = byte_parser_snd_bank_name_load(script_bytes);
                break;
            case 0x12u:
                script_bytes = music_voc_name_patcher(script_bytes);
                break;
            }
            if ((byte_parser_b16_flag & 1u) != 0u) {
                break;
            }
        }
        result = 1;
    }

    *(cb_game_word_ptr)byte_parser_stream_0f18_cursor = 0xFFFFu;
    byte_parser_stream_0f18_cursor =
            (cb_game_char_ptr)BYTE_PARSER_STREAM_BASE;
    cb_dos_close(file_handle);
    return result;
}
