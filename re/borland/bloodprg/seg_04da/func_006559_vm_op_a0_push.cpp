// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x006559
// assembly: re/assembly/bloodprg/seg_04da/func_006559_vm_op_a0_push.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_a0_push
// reason: mechanical translation of VM stack push handler

#include "recovered.hpp"

// label: vm_op_a0_push

extern "C" void CB_NEAR cb_bloodprg_006559_vm_op_a0_push(CbMachine* m)
{
    m->write8(m->gs, 0x67ad, 1);
    m->ax = m->read16(m->gs, 0x6884);
    m->bp = m->ax;
    cb_u16 before_add = m->ax;
    m->ax = (cb_u16)(m->ax + 2);
    m->set_add16_flags(before_add, 2, m->ax);
    m->write16(m->gs, 0x6884, m->ax);
    m->ax = m->read16(m->ds, m->si);
    cb_advance_u16(m->si, 2, m->df);
    m->write16(m->ss, (cb_u16)(m->bp + 0x6820), m->ax);
    return;
}
