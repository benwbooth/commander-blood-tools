#include "../include/bloodprg_ems.h"
#include "../include/bloodprg_resource.h"

void CB_NEAR snd_bank_file_page_read(cb_u16 page,
        volatile cb_u8 CB_FAR *destination)
{
    cb_u32 offset;
    cb_u16 handle;

    offset = (cb_u32)page << 14;
    handle = snd_bank_file_handle;
    cb_dos_seek_absolute(handle, offset);
    (void)cb_dos_read(handle, destination, 0x4000u);
}
