#include "../include/bloodprg_byte_parser.h"
#include "../include/bloodprg_vm.h"

void CB_NEAR index_lookup_1fd7(const cb_u8 **script_bytes)
{
    volatile char *dst;
    cb_u8 id;
    cb_u8 ch;

    id = **script_bytes;
    ++*script_bytes;
    if ((id & 0x80u) != 0) {
        byte_parser_index_word_1fd7 = (cb_u16)(int)(cb_i8)id;
    } else {
        byte_parser_index_word_1fd7 =
                (cb_u16)(0x0dd7u + (((cb_u16)id - 1u) << 4));
    }

    dst = byte_parser_index_text_213a;
    for (;;) {
        ch = **script_bytes;
        if ((ch & 0x80u) != 0 || ch < 0x20u) {
            break;
        }
        *dst = (char)ch;
        ++dst;
        ++*script_bytes;
    }
    *dst = '\0';

    if ((vm_ui_flags & 1u) == 0) {
        if (byte_parser_ems_handle_a58 != -1) {
            path_build_call_2693(byte_parser_index_path_2137);
        } else if (byte_parser_ems_handle_a56 != -1) {
            file_open_wrapper(byte_parser_index_path_2137,
                    byte_parser_back_buffer);
        }
    }
}
