#include "../include/bloodprg_ship3d.h"

void CB_FAR matrix_table_clear_2a1b(void)
{
    cb_u16 i;

    for (i = 0; i < 6u; ++i) {
        ship_3d_matrix_slots[i].first_word = 0;
    }
}
