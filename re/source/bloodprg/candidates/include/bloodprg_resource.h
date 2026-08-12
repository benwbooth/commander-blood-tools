#ifndef BLOODPRG_RESOURCE_H
#define BLOODPRG_RESOURCE_H

#include "bloodprg_common.h"

typedef struct bloodprg_resource_handle_entry {
    cb_u16 unknown_00;
    cb_u16 unknown_02;
    cb_u32 field_04;
} bloodprg_resource_handle_entry;

extern const volatile bloodprg_resource_handle_entry fs_resource_handle_table[]; /* FS:0x0000 */

cb_u32 CB_FAR resource_file_load(const volatile char *path,
        volatile cb_u8 CB_FAR *destination); /* 0x01CE:0x07DB */

#endif
