/*
 * Codegen probe for BLOODPRG 0x007542/0x007549/0x007550/0x007557.
 * This is not recovered game source.
 */
typedef unsigned char u8;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u8 parser_b16_flag;

#if defined(__WATCOMC__)
#pragma aux byte_parser_mark_b16_probe modify exact []
#endif

void NEAR byte_parser_mark_b16_probe(void)
{
    parser_b16_flag = 1;
}
