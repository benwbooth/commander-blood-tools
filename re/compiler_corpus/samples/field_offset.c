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

i16 NEAR field_offset_probe(u16 selector, u16 kind_mask)
{
    u16 bit_index;

    bit_index = 0;
    while ((kind_mask & (u16)(1u << bit_index)) == 0) {
        ++bit_index;
    }

    return (i16)selector_field_offsets[(u16)((selector << 4) + bit_index)];
}
