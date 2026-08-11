// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0065db
// assembly: re/assembly/bloodprg/seg_04da/func_0065db_vm_op_a4_jump.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_a4_jump
// reason: mechanical translation of VM jump handler

#include "recovered.hpp"

// label: vm_op_a4_jump

extern "C" void CB_NEAR cb_bloodprg_0065db_vm_op_a4_jump(CbMachine* m)
{
    m->si = m->read16(m->ds, m->si);
    m->write8(m->gs, 0x67b1, 0);
    m->write16(m->gs, 0x6764, 0);
    return;
}
