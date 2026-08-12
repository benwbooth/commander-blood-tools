#include "../include/bloodprg_list.h"

int CB_NEAR list_d8c_state_le_one(void)
{
    if (list_d8c_state_byte == 0) {
        return 1;
    }
    return list_d8c_state_byte == 1;
}
