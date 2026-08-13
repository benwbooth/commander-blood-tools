/* Codegen probe for BLOODPRG 0x0024EB. */

typedef unsigned char u8;
typedef unsigned long u32;
typedef signed long i32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

#if defined(__WATCOMC__)
#define CODE_DATA __based(__segname("_CODE"))
#else
#define CODE_DATA FAR
#endif

#define DECIMAL_SCRATCH_END 11u

extern u8 CODE_DATA decimal_append_scratch[12];

void FAR decimal_append_i32_probe(i32 value, char FAR *destination);

#if defined(__WATCOMC__)
#pragma aux decimal_append_i32_probe parm [dx ax] [es di] modify exact []
#endif

void FAR decimal_append_i32_probe(i32 value, char FAR *destination)
{
    u8 CODE_DATA *cursor;
    u32 magnitude;
    u32 quotient;

    cursor = decimal_append_scratch + DECIMAL_SCRATCH_END;
    if (value < 0) {
        *destination++ = '-';
        magnitude = 0UL - (u32)value;
    } else {
        magnitude = (u32)value;
    }

    do {
        quotient = magnitude / 10UL;
        *--cursor = (u8)('0' + (u8)(magnitude - quotient * 10UL));
        magnitude = quotient;
    } while (magnitude != 0);

    do {
        *destination++ = (char)*cursor;
    } while (*cursor++ != 0);
}
