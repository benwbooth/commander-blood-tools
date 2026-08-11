// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0064c0
// assembly: re/assembly/bloodprg/seg_04da/func_0064c0_vm_op_cf_clear_state.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_cf_clear_state
// reason: mechanical translation of two GS state-clearing stores

#include "recovered.hpp"

// label: vm_op_cf_clear_state

extern "C" void CB_NEAR cb_bloodprg_0064c0_vm_op_cf_clear_state(CbMachine* m)
{
    m->write8(m->gs, 0x67b1, 0);
    m->write16(m->gs, 0x6764, 0);
    return;
}
