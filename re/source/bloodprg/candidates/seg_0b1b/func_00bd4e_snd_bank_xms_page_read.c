#include "../include/bloodprg_ems.h"

void CB_NEAR snd_bank_xms_page_read(cb_u16 page,
        volatile cb_u8 CB_FAR *destination)
{
    xms_move_request.length = 0x4000u;
    xms_move_request.source_handle = (cb_u16)snd_bank_xms_handle;
    xms_move_request.source.offset = (cb_u32)page << 14;
    xms_move_request.destination_handle = 0;
    xms_move_request.destination.pointer = destination;
    cb_xms_move(&xms_move_request);
}
