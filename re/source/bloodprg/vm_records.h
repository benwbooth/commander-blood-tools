#ifndef CB_BLOODPRG_VM_RECORDS_H
#define CB_BLOODPRG_VM_RECORDS_H

#include "cb_types.h"

void CB_NEAR cb_bloodprg_006fb9_vm_op_c9_clear_record_full(
    cb_u8 CB_FAR *record_heap,
    cb_u16 record_off,
    const cb_i8 CB_FAR *selector_field_offsets,
    cb_u8 CB_FAR *nav_state_252a,
    cb_u8 CB_FAR *nav_state_2531);

#endif
