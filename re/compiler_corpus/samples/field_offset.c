/*
 * Codegen probe for BLOODPRG 0x006023.
 * This is not recovered game source.
 */
typedef signed char i8;
typedef signed int i16;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern const i8 FAR selector_field_offsets[];

#if defined(__WATCOMC__)
#pragma aux field_offset_probe parm [ax] [bx] value [ax] modify exact [ax]
#endif

i16 NEAR field_offset_probe(u16 selector, u16 kind_mask)
{
    u16 bit_index;

    bit_index = 0;
    while ((kind_mask & 1u) == 0) {
        kind_mask >>= 1;
        ++bit_index;
    }

    return (i16)selector_field_offsets[(u16)((selector << 4) + bit_index)];
}
