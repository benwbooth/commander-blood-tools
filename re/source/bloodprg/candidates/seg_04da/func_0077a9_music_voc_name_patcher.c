#include "../include/bloodprg_byte_parser.h"

void CB_NEAR music_voc_name_patcher(const cb_u8 **script_bytes)
{
    volatile char *dst;
    cb_u8 ch;

    dst = music_voc_name_field;
    for (;;) {
        ch = **script_bytes;
        if ((ch & 0x80u) != 0 || ch <= 0x20u) {
            break;
        }
        if (ch >= 'a') {
            ch = (cb_u8)(ch & 0xdfu);
        }
        if (ch != (cb_u8)*dst) {
            music_voc_name_changed = 1;
        }
        *dst = (char)ch;
        ++dst;
        ++*script_bytes;
    }

    if ((music_voc_name_changed & 1u) == 0) {
        music_voc_name_unchanged |= 1;
    }
    *dst = '\0';
}
