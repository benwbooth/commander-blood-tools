#include "../include/bloodprg_byte_parser.h"

void CB_NEAR byte_parser_store_word_1fa5(const cb_u16 **script_words)
{
    byte_parser_word_1fa5 = **script_words;
    ++*script_words;
}
