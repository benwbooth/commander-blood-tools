#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_FAR *CB_NEAR fs_name_area_read(
    const cb_u8 CB_FAR *script_bytes)
{
    char CB_FS_DATA *dst;
    cb_u8 ch;

    dst = fs_resource_name_area;
    for (;;) {
        ch = *script_bytes++;
        if ((cb_i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        *dst++ = (char)ch;
    }

    *dst = '\0';
    fs_name_area_dirty = 1;
    return script_bytes;
}
