#include "../include/bloodprg_audio.h"
#include "../include/bloodprg_byte_parser.h"
#include "../include/bloodprg_vm.h"

void CB_NEAR byte_parser_snd_bank_name_load(const cb_u8 **script_bytes)
{
    volatile char *dst;
    cb_u8 ch;

    dst = byte_parser_snd_bank_name_field;
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
        snd_bank_loader(1u, byte_parser_snd_bank_path);
    }
}
