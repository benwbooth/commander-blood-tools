#include "../include/bloodprg_byte_parser.h"

void CB_NEAR byte_parser_op_04_mark_b16(void)
{
#if defined(__WATCOMC__)
    bloodprg_byte_parser_mark_b16_gs();
#else
    byte_parser_b16_flag = 1;
#endif
}
