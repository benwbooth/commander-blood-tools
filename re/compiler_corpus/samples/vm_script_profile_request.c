/*
 * Codegen probe for BLOODPRG 0x0064B8.
 * This is not recovered game source.
 */
typedef signed char i8;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile i16 script_profile_request;

#if defined(__WATCOMC__)
#pragma aux vm_script_profile_request_probe parm [si] value [si] modify exact [ax si]
#endif

const i8 NEAR *NEAR vm_script_profile_request_probe(
        const i8 NEAR *script_bytes)
{
    i16 request;

    request = (int)*script_bytes++ - 1;
    script_profile_request = request;
    return script_bytes;
}
