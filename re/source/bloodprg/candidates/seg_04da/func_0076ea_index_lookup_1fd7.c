#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_FAR *CB_NEAR index_lookup_1fd7(
    const cb_u8 CB_FAR *script_bytes)
{
    cb_u16 stored_id;
    cb_u16 dst_index;
    cb_u8 ch;

    stored_id = (cb_u16)(cb_i16)(cb_i8)*script_bytes++;
    /* CBW inherits clear SF from the opcode-0x0B dispatch-table index. */
    stored_id = (cb_u16)(0x0dd7u + ((stored_id - 1u) << 4));
    byte_parser_index_word_1fd7 = stored_id;

    dst_index = 0u;
    for (;;) {
        ch = *script_bytes++;
        if ((cb_i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        byte_parser_index_text_213a[dst_index++] = (char)ch;
    }
    byte_parser_index_text_213a[dst_index] = '\0';

    if ((byte_parser_ui_state & 1u) == 0) {
        if (resource_ems_handle != -1) {
            resource_file_load_to_ems(byte_parser_index_path_2137);
        } else if (resource_xms_handle != -1) {
            resource_file_load_to_xms(byte_parser_index_path_2137,
                    byte_parser_back_buffer);
        }
    }
    return script_bytes;
}
