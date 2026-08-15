/*
 * Codegen probe for BLOODPRG 0x0065EB.
 * This is not recovered game source.
 */
typedef signed char i8;
typedef signed int i16;
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__WATCOMC__)
#define NEAR near
#define GAME_DATA __based(__segname("GAME_DATA"))
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#define NEAR near
#define GAME_DATA far
#else
#define NEAR
#define GAME_DATA
#endif

extern volatile u8 GAME_DATA query_mode;
extern volatile u16 GAME_DATA state_words[];
u16 NEAR vm_branch_probe(void);

#if defined(__WATCOMC__)
#pragma aux vm_branch_probe value [si] modify exact [ax si]
#pragma aux vm_cond_state_array_probe parm [si] value [si] modify exact [ax bp si]
#endif

const u8 NEAR *NEAR vm_cond_state_array_probe(
        const u8 NEAR *script_bytes)
{
    i16 index;

    index = (i8)*script_bytes++;
    if ((query_mode & 1u) != 0) {
        if (state_words[index] != 0) {
            return (const u8 NEAR *)vm_branch_probe();
        }
    } else {
        state_words[index] = *(const u16 NEAR *)script_bytes;
        script_bytes += sizeof(u16);
    }

    return script_bytes;
}
