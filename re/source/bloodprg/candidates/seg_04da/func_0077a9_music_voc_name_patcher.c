#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_NEAR *CB_NEAR music_voc_name_patcher(
    const cb_u8 CB_NEAR *script_bytes)
{
    char CB_GAME_DATA *dst;
    cb_u8 ch;

    dst = music_voc_name_field;
    for (;;) {
        ch = *script_bytes++;
        if ((cb_i8)ch < 0 || ch <= 0x20u) {
            --script_bytes;
            break;
        }
        if (ch >= 0x61u) {
            ch = (cb_u8)(ch & 0xdfu);
        }
        if (ch != (cb_u8)*dst) {
            music_voc_name_changed = 1;
        }
        *dst++ = (char)ch;
    }

    if ((music_voc_name_changed & 1u) == 0) {
        music_voc_name_unchanged |= 1;
    }
    *dst = '\0';
    return script_bytes;
}
