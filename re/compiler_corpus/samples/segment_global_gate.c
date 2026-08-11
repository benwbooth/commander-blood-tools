/*
 * Codegen probe for BLOODPRG 0x006494.
 * This is not recovered game source.
 */
typedef unsigned char u8;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern u8 presentation_flags;
void NEAR vm_branch_probe(void);

void NEAR vm_op_ce_gate_probe(void)
{
    if ((presentation_flags & 1u) == 0) {
        vm_branch_probe();
    }
}
