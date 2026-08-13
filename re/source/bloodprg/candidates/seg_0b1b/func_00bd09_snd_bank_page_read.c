#include "../include/bloodprg_ems.h"

void CB_NEAR snd_bank_page_read(cb_u16 page,
        volatile cb_u8 CB_FAR *destination)
{
    cb_i8 mode;

    mode = (cb_i8)snd_bank_storage_mode;
    if (--mode < 0) {
        snd_bank_ems_page_read(page, destination);
    } else {
        if (--mode < 0) {
            snd_bank_xms_page_read(page, destination);
        } else {
            snd_bank_file_page_read(page, destination);
        }
    }
}
