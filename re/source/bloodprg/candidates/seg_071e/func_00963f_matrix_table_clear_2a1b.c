#include "../include/bloodprg_ship3d.h"

void CB_FAR matrix_table_clear_2a1b(void)
{
    volatile ship_3d_matrix_slot CB_NEAR *slot;

    slot = ship_3d_matrix_slots;
    do {
        slot->first_word = 0;
        ++slot;
    } while (slot != ship_3d_matrix_slots + 6u);
}
