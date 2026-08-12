/* Codegen probe for the MANU3 far animation-selector wrapper. */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern void NEAR xdb_manu3_anim_select_probe(u16 selector);
void FAR xdb_manu3_anim_select_entry_probe(u16 selector);

#if defined(__WATCOMC__)
#pragma aux xdb_manu3_anim_select_probe \
        parm [bx] modify exact [ax bx cx dx si di bp]
#pragma aux xdb_manu3_anim_select_entry_probe \
        parm [bx] modify exact [ax bx cx dx si di bp]
#endif

void FAR xdb_manu3_anim_select_entry_probe(u16 selector)
{
    xdb_manu3_anim_select_probe(selector);
}
