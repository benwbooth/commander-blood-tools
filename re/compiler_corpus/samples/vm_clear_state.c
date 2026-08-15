/*
 * Codegen probe for BLOODPRG 0x0064C0.
 * This is not recovered game source.
 */
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

extern volatile u8 GAME_DATA resume_state;
extern volatile u16 GAME_DATA resume_value;

void NEAR vm_clear_state_probe(void)
{
    resume_state = 0;
    resume_value = 0;
}
