/*
 * Codegen probe for BLOODPRG 0x00684C.
 * This is not recovered game source.
 */
typedef unsigned char u8;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

#if defined(__WATCOMC__)
#pragma aux vm_poke_byte_probe parm [si] value [si] modify exact [ax bx si]
#endif

const u8 NEAR *NEAR vm_poke_byte_probe(const u8 NEAR *script_bytes)
{
    u8 value;
    volatile u8 NEAR *target;

    value = *script_bytes++;
    target = *(volatile u8 NEAR * const NEAR *)script_bytes;
    *target = value;
    script_bytes += sizeof(target);
    return script_bytes;
}
