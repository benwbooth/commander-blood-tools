/*
 * Codegen probe for BLOODPRG 0x006494.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern u8 presentation_flags;
u16 NEAR vm_branch_probe(void);

#if defined(__WATCOMC__)
#pragma aux vm_branch_probe value [si] modify exact [ax si]
#pragma aux vm_op_ce_gate_probe modify exact [ax si]
#endif

void NEAR vm_op_ce_gate_probe(void)
{
    if ((presentation_flags & 1u) == 0) {
        vm_branch_probe();
    }
}
