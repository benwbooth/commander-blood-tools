/*
 * Codegen probe for BLOODPRG 0x0064CE.
 * This is not recovered game source.
 */
typedef signed char i8;
typedef unsigned char u8;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile char record_string_slots[][16];

#if defined(__WATCOMC__)
#pragma aux vm_record_string_copy_probe parm [si] value [si] modify exact [ax si]
#endif

const u8 NEAR *NEAR vm_record_string_copy_probe(
        const u8 NEAR *script_bytes)
{
    i8 slot;
    volatile char NEAR *destination;
    u8 character;

    slot = (i8)(u8)(*script_bytes++ - 1u);
    destination = (volatile char NEAR *)record_string_slots
        + (i16)slot * 16;

    do {
        character = *script_bytes++;
        *destination++ = (char)character;
    } while (character != 0);

    return script_bytes + 1;
}
