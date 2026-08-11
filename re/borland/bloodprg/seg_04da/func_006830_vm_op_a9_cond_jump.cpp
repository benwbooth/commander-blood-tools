// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x006830
// assembly: re/assembly/bloodprg/seg_04da/func_006830_vm_op_a9_cond_jump.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_a9_cond_jump
// reason: mechanical translation of VM A9 conditional jump handler

#include "recovered.hpp"

// label: vm_op_a9_cond_jump

extern "C" void CB_NEAR cb_bloodprg_006830_vm_op_a9_cond_jump(CbMachine* m)
{
    cb_set_lo8(m->ax, m->read8(m->ds, m->si));
    cb_advance_u16(m->si, 1, m->df);
    cb_u8 test_result = (cb_u8)(cb_lo8(m->ax) & 1);
    m->set_logic8_flags(test_result);
    if (test_result == 0) {
        m->si = m->read16(m->ds, m->si);
        return;
    }
    m->write8(m->gs, 0x67ad, 1);
    m->ax = m->read16(m->ds, m->si);
    cb_advance_u16(m->si, 2, m->df);
    m->write16(m->gs, 0x6820, m->ax);
    m->write16(m->gs, 0x6884, 2);
    return;
}
