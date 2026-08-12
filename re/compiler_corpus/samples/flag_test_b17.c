/*
 * Codegen probe for BLOODPRG 0x00A634.
 * This is not recovered game source.
 */
typedef unsigned char u8;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u8 state_flag_b17;

int NEAR flag_test_b17_probe(void)
{
    return (state_flag_b17 & 1u) != 0;
}
