#ifndef CB_BLOODPRG_OBJECT_HEAP_H
#define CB_BLOODPRG_OBJECT_HEAP_H

#include "cb_types.h"

void CB_NEAR cb_bloodprg_00149b_object_heap_access(
    cb_u8 CB_FAR *object_heap,
    const cb_u8 CB_FAR *lookup_table);

void CB_NEAR cb_bloodprg_00604e_active_object_list_build(
    const cb_u8 CB_FAR *lookup_table,
    const cb_u8 CB_FAR *object_heap,
    cb_u16 CB_FAR *out_object_offsets);

#endif
