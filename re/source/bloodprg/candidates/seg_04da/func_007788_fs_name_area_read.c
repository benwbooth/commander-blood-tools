#include "../include/bloodprg_byte_parser.h"

void CB_NEAR fs_name_area_read(const cb_u8 **script_bytes)
{
    volatile char *dst;
    cb_u8 ch;

    dst = fs_resource_name_area;
    for (;;) {
        ch = **script_bytes;
        if ((ch & 0x80u) != 0 || ch < 0x20u) {
            break;
        }
        *dst = (char)ch;
        ++dst;
        ++*script_bytes;
    }

    *dst = '\0';
    fs_name_area_dirty = 1;
}
