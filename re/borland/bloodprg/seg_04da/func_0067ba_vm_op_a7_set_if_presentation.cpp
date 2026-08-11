// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0067ba
// assembly: re/assembly/bloodprg/seg_04da/func_0067ba_vm_op_a7_set_if_presentation.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_a7_set_if_presentation
// reason: mechanical translation of conditional presentation-state store

#include "recovered.hpp"

// label: vm_op_a7_set_if_presentation

extern "C" void CB_NEAR cb_bloodprg_0067ba_vm_op_a7_set_if_presentation(CbMachine* m)
{
    m->ax = m->read16(m->ds, m->si);
    cb_advance_u16(m->si, 2, m->df);
    cb_u8 test_result = (cb_u8)(m->read8(m->gs, 0x67ac) & 1);
    m->set_logic8_flags(test_result);
    if (test_result != 0) {
        m->write16(m->gs, 0x6770, m->ax);
    }
    return;
}
