/*
 * Codegen probe for BLOODPRG 0x0065DB.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u8 resume_state;
extern volatile u16 resume_value;

#if defined(__WATCOMC__)
#pragma aux vm_script_jump_probe parm [si] value [si] modify exact [si]
#endif

const u8 NEAR *NEAR vm_script_jump_probe(const u16 NEAR *script_words)
{
    const u8 NEAR *target;

    target = (const u8 NEAR *)*script_words;
    resume_state = 0;
    resume_value = 0;
    return target;
}
