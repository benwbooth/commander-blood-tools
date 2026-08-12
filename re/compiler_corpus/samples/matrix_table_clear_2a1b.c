/*
 * Codegen probe for BLOODPRG 0x00963F.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

typedef struct matrix_slot {
    u16 first_word;
    u8 tail[22];
} matrix_slot;

extern volatile matrix_slot matrix_slots[];

#if defined(__WATCOMC__)
#pragma aux matrix_table_clear_2a1b_probe modify exact []
#endif

void FAR matrix_table_clear_2a1b_probe(void)
{
    volatile matrix_slot *slot;

    slot = matrix_slots;
    do {
        slot->first_word = 0;
        ++slot;
    } while (slot != matrix_slots + 6u);
}
