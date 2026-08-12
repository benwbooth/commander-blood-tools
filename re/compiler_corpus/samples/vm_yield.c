/*
 * Codegen probe for BLOODPRG 0x006855 and 0x00685C.
 * This is not recovered game source.
 */
typedef unsigned char u8;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u8 yield_flag;

void NEAR vm_yield_probe(void)
{
    yield_flag = 1;
}
