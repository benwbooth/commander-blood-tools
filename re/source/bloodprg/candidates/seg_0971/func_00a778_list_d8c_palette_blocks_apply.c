#include <dos.h>

#include "../include/bloodprg_list.h"

volatile cb_u8 CB_FAR *CB_NEAR list_d8c_palette_blocks_apply(void)
{
    cb_u16 queue_segment;
    cb_u16 palette_offset;

    queue_segment = list_d8c_head_segment;
    palette_offset = list_d8c_palette_offset;
    return resource_palette_blocks_apply((volatile cb_u8 CB_FAR *)MK_FP(
            queue_segment, palette_offset));
}
