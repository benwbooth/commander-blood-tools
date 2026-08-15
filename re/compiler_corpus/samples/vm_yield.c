/*
 * Codegen probe for BLOODPRG 0x006855 and 0x00685C.
 * This is not recovered game source.
 */
typedef unsigned char u8;

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

extern volatile u8 GAME_DATA yield_flag;

void NEAR vm_yield_probe(void)
{
    yield_flag = 1;
}
