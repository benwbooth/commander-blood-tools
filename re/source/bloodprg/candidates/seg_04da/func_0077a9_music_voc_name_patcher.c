#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_FAR *CB_NEAR music_voc_name_patcher(
    const cb_u8 CB_FAR *script_bytes)
{
    cb_u16 dst_index;
    cb_u8 ch;

    dst_index = 0u;
    for (;;) {
        ch = *script_bytes++;
        if ((cb_i8)ch < 0 || ch <= 0x20u) {
            --script_bytes;
            break;
        }
        if (ch >= 0x61u) {
            ch = (cb_u8)(ch & 0xdfu);
        }
        if (ch != (cb_u8)music_voc_name_field[dst_index]) {
            music_voc_name_changed = 1;
        }
        music_voc_name_field[dst_index++] = (char)ch;
    }

    if ((music_voc_name_changed & 1u) == 0) {
        music_voc_name_unchanged |= 1;
    }
    music_voc_name_field[dst_index] = '\0';
    return script_bytes;
}
