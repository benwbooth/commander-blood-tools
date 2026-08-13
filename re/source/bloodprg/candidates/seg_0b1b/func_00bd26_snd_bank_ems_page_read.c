#include <dos.h>
#include <string.h>

#include "../include/bloodprg_ems.h"

#if defined(__WATCOMC__)
#pragma intrinsic(_fmemcpy)
#endif

void CB_NEAR snd_bank_ems_page_read(cb_u16 page,
        volatile cb_u8 CB_FAR *destination)
{
    cb_u16 page_frame;
    cb_u16 handle;

    page_frame = ems_page_frame_segment;
    handle = (cb_u16)snd_bank_ems_handle;
    cb_ems_map_page(handle, page, 0u);
    _fmemcpy(
            (void CB_FAR *)destination,
            (const void CB_FAR *)MK_FP(page_frame, 0u),
            0x4000u);
}
