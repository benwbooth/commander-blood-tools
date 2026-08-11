// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0027c3
// assembly: re/assembly/bloodprg/seg_01ce/func_0027c3_set_ds_gs_check_ae0.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_set_ds_gs_check_ae0
// reason: mechanical translation of GS DOS-drive setup gate preserving AX/DX/DS

#include "recovered.hpp"

// label: set_ds_gs_check_ae0

extern "C" void CB_FAR cb_bloodprg_0027c3_set_ds_gs_check_ae0(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->ds);
    m->push16(m->dx);
    m->ax = m->gs;
    m->ds = m->ax;
    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x0ae0) & 1);
    m->set_logic8_flags(test_result);
    if (test_result == 0) {
        cb_set_hi8(m->ax, 0x0e);
        cb_set_lo8(m->dx, m->read8(m->ds, 0x01b8));
        m->interrupt(0x21);
        m->dx = 0x01ba;
        cb_set_hi8(m->ax, 0x3b);
        m->interrupt(0x21);
        m->write8(m->ds, 0x0ae0, 1);
    }
    m->dx = m->pop16();
    m->ds = m->pop16();
    m->ax = m->pop16();
    return;
}
