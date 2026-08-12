#include "../include/bloodprg_byte_parser.h"

void CB_NEAR credit_presenter_b_cryo(const char **script_bytes)
{
    volatile char *dst;
    char ch;

    dst = credit_text_buffer;
    do {
        ch = **script_bytes;
        ++*script_bytes;
        *dst = ch;
        ++dst;
    } while (ch != '\0');

    credit_reveal_active = 1;
    credit_reveal_timer = 0;
}
