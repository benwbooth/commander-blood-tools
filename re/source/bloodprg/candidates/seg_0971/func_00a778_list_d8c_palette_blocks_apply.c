#include "../include/bloodprg_list.h"

volatile cb_u8 CB_FAR *CB_NEAR list_d8c_palette_blocks_apply(void)
{
    return resource_palette_blocks_apply(
            list_d8c_buffer + list_d8c_palette_offset);
}
