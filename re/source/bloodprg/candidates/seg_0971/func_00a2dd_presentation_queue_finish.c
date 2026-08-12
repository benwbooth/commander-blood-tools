#include "../include/bloodprg_list.h"

void CB_NEAR presentation_queue_finish(void)
{
    list_d8c_state_byte |= 1u;
    if (list_d8c_byte_count == 0) {
        list_d8c_state_byte |= 2u;
        close_file_d5b();
    }
}
