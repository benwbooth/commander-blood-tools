/*
 * Codegen probe for BLOODPRG 0x005FF6.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u16 special_slots[16];

int NEAR vm_special_slot_insert_probe(u16 owner)
{
    unsigned i;

    for (i = 0; i != 16u; ++i) {
        if (special_slots[i] == owner) {
            return 1;
        }
    }

    for (i = 0; i != 16u; ++i) {
        if (special_slots[i] == 0) {
            special_slots[i] = owner;
            return 1;
        }
    }

    return 0;
}
