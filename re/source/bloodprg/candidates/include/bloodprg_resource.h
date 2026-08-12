#ifndef BLOODPRG_RESOURCE_H
#define BLOODPRG_RESOURCE_H

#include "bloodprg_common.h"

typedef struct bloodprg_resource_handle_entry {
    cb_u16 unknown_00;
    cb_u16 unknown_02;
    cb_u32 field_04;
} bloodprg_resource_handle_entry;

extern const volatile bloodprg_resource_handle_entry fs_resource_handle_table[]; /* FS:0x0000 */

#endif
