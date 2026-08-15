#include "../include/bloodprg_byte_parser.h"

const cb_u8 CB_FAR *CB_NEAR fs_name_area_read(
    const cb_u8 CB_FAR *script_bytes)
{
    cb_u16 dst_index;
    cb_u8 ch;

    dst_index = 0u;
    for (;;) {
        ch = *script_bytes++;
        if ((cb_i8)ch < 0 || ch < 0x20u) {
            --script_bytes;
            break;
        }
        fs_resource_name_area[dst_index++] = (char)ch;
    }

    fs_resource_name_area[dst_index] = '\0';
    fs_name_area_dirty = 1;
    return script_bytes;
}
