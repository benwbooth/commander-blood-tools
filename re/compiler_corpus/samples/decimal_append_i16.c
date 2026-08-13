/* Codegen probe for BLOODPRG 0x0024B2. */

typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

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

u8 CODE_DATA decimal_append_scratch[12] = {0};

void FAR decimal_append_i16_probe(i16 value, char FAR *destination);

#if defined(__WATCOMC__)
#pragma aux decimal_append_i16_probe parm [ax] [es di] modify exact []
#endif

void FAR decimal_append_i16_probe(i16 value, char FAR *destination)
{
    u8 CODE_DATA *cursor;
    u16 magnitude;
    u16 quotient;

    cursor = decimal_append_scratch + DECIMAL_SCRATCH_END;
    if (value < 0) {
        *destination++ = '-';
        magnitude = (u16)(0u - (u16)value);
    } else {
        magnitude = (u16)value;
    }

    do {
        quotient = magnitude / 10u;
        *--cursor = (u8)('0' + magnitude - quotient * 10u);
        magnitude = quotient;
    } while (magnitude != 0);

    do {
        *destination++ = (char)*cursor;
    } while (*cursor++ != 0);
}
