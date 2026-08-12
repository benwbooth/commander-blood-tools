#include "../include/bloodprg_ems.h"

void CB_NEAR ems_transfer_dispatch(cb_u16 value,
        volatile cb_u8 CB_FAR *destination)
{
    cb_u8 mode;

    mode = ems_transfer_mode;
    if (mode == 0u || mode > 128u) {
        ems_map_page_and_copy(value, destination);
    } else if (mode == 1u) {
        ems_buffer_setup(value, destination);
    } else {
        ems_page_offset_split(value, destination);
    }
}
