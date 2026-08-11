// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x001fbc
// assembly: re/assembly/bloodprg/seg_008b/func_001fbc_flag_test_a2e.asm
// provenance: recursive_graph
// status: translated_flag_test_a2e
// reason: mechanical translation of two flag-bit propagation blocks

#include "recovered.hpp"

// label: flag_test_a2e

extern "C" void CB_NEAR cb_bloodprg_001fbc_flag_test_a2e(CbMachine* m)
{
    m->ax = m->read16(m->ds, 0x0a2e);
    cb_u8 al = cb_lo8(m->ax);
    cb_u8 test1 = (cb_u8)(al & 1);
    m->set_logic8_flags(test1);
    if (test1 != 0) {
        al = (cb_u8)(al & m->read8(m->ds, 0x0a30));
        cb_set_lo8(m->ax, al);
        m->set_logic8_flags(al);
        if (al == 0) {
            m->write8(m->ds, 0x0a3e, 1);
            m->write8(m->ds, 0x0a40, 1);
        }
    }
    cb_u8 test2 = (cb_u8)(al & 2);
    m->set_logic8_flags(test2);
    if (test2 != 0) {
        al = (cb_u8)(al & m->read8(m->ds, 0x0a30));
        cb_set_lo8(m->ax, al);
        m->set_logic8_flags(al);
        if (al == 0) {
            m->write8(m->ds, 0x0a3f, 1);
            m->write8(m->ds, 0x0a40, 1);
        }
    }
    m->ax = m->read16(m->ds, 0x0a2e);
    m->write16(m->ds, 0x0a30, m->ax);
    return;
}
