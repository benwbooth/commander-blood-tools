/* Codegen probe for BLOODPRG 0x005FD8. */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u16 special_slots[16];

#if defined(__WATCOMC__)
#pragma aux vm_special_slot_remove_probe parm [ax] value [ax] modify exact [ax]
#endif

int NEAR vm_special_slot_remove_probe(u16 owner)
{
    u16 i;

    for (i = 0; i < 16u; ++i) {
        if (special_slots[i] == owner) {
            special_slots[i] = 0;
            return 1;
        }
    }

    return 0;
}
