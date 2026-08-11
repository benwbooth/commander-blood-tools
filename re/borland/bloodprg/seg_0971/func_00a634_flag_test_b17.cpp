// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a634
// assembly: re/assembly/bloodprg/seg_0971/func_00a634_flag_test_b17.asm
// provenance: recursive_graph
// status: translated_flag_test_b17
// reason: mechanical translation of DS=GS flag-byte test preserving AX/DS

#include "recovered.hpp"

// label: flag_test_b17

extern "C" void CB_NEAR cb_bloodprg_00a634_flag_test_b17(CbMachine* m)
{
    cb_u16 saved_ax = m->ax;
    cb_u16 saved_ds = m->ds;
    m->ax = m->gs;
    m->ds = m->ax;
    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x0b17) & 1);
    m->set_logic8_flags(test_result);
    m->ds = saved_ds;
    m->ax = saved_ax;
    return;
}
