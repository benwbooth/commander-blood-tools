/*
 * Codegen probe for BLOODPRG 0x0064C0.
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

void NEAR vm_clear_state_probe(void)
{
    resume_state = 0;
    resume_value = 0;
}
