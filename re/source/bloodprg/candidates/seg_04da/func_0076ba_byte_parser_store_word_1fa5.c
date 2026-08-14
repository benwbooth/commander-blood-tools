#include "../include/bloodprg_byte_parser.h"

const cb_u16 CB_FAR *CB_NEAR byte_parser_store_word_1fa5(
    const cb_u16 CB_FAR *script_words)
{
    byte_parser_word_1fa5 = *script_words++;
    return script_words;
}
