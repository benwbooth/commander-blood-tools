/* Codegen probe for BLOODPRG 0x000986. */

typedef unsigned char u8;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

u8 NEAR bcd_to_binary_probe(u8 value);

#if defined(__WATCOMC__)
#pragma aux bcd_to_binary_probe parm [ax] value [al] modify [ax]
#endif

u8 NEAR bcd_to_binary_probe(u8 value)
{
    u8 low;
    u8 high;

    low = (u8)(value & 0x0Fu);
    high = (u8)(value >> 4);
    return (u8)((high * 10u) + low);
}
