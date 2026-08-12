#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_NEAR *CB_NEAR byte_parser_snd_bank_name_load(
    const cb_u8 CB_NEAR *script_bytes)
{
    char CB_GAME_DATA *dst;
    cb_u8 ch;

    dst = byte_parser_snd_bank_name_field;
    for (;;) {
        ch = *script_bytes++;
        if ((cb_i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        *dst++ = (char)ch;
    }
    *dst = '\0';

    if ((byte_parser_ui_state & 1u) == 0) {
        snd_bank_loader(1u, byte_parser_snd_bank_path);
    }
    return script_bytes;
}
