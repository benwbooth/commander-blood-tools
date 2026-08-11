// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0027e9
// assembly: re/assembly/bloodprg/seg_01ce/func_0027e9_dos_set_drive_and_chdir.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_dos_set_drive_and_chdir
// reason: mechanical translation of GS DOS-drive restore gate preserving AX/DX/DS

#include "recovered.hpp"

// label: dos_set_drive_and_chdir

extern "C" void CB_FAR cb_bloodprg_0027e9_dos_set_drive_and_chdir(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->ds);
    m->push16(m->dx);
    m->ax = m->gs;
    m->ds = m->ax;
    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x0ae0) & 1);
    m->set_logic8_flags(test_result);
    if (test_result != 0) {
        cb_set_hi8(m->ax, 0x0e);
        cb_set_lo8(m->dx, m->read8(m->ds, 0x01b9));
        m->interrupt(0x21);
        m->dx = 0x01da;
        cb_set_hi8(m->ax, 0x3b);
        m->interrupt(0x21);
        m->write8(m->ds, 0x0ae0, 0);
    }
    m->dx = m->pop16();
    m->ds = m->pop16();
    m->ax = m->pop16();
    return;
}
