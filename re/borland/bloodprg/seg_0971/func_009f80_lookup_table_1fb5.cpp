// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x009f80
// assembly: re/assembly/bloodprg/seg_0971/func_009f80_lookup_table_1fb5.asm
// provenance: recursive_graph
// status: translated_lookup_table_1fb5
// reason: mechanical translation of AX*4 table lookup at DS:0x1fb5

#include "recovered.hpp"

// label: lookup_table_1fb5

extern "C" void CB_NEAR cb_bloodprg_009f80_lookup_table_1fb5(CbMachine* m)
{
    m->bx = 0x1fb5;
    for (int i = 0; i != 4; ++i) {
        cb_u16 before_add = m->bx;
        m->bx = (cb_u16)(m->bx + m->ax);
        m->set_add16_flags(before_add, m->ax, m->bx);
    }
    m->bx = m->read16(m->ds, m->bx);
    return;
}
