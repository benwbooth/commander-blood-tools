/*
 * Codegen probe for BLOODPRG 0x006293.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

#if defined(__WATCOMC__)
#pragma aux vm_token_special_probe parm [ax] [si] value [si] modify exact [si]
#endif

const u8 NEAR *NEAR vm_token_special_probe(u16 terminator,
        const u8 NEAR *script_bytes)
{
    while (*(const u16 NEAR *)script_bytes != terminator) {
        ++script_bytes;
    }

    script_bytes += 2;
    if (*script_bytes == (u8)terminator) {
        ++script_bytes;
    }

    return script_bytes;
}
