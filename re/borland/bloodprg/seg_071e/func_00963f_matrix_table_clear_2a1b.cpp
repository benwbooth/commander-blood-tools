// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00963f
// assembly: re/assembly/bloodprg/seg_071e/func_00963f_matrix_table_clear_2a1b.asm
// provenance: recursive_graph
// status: translated_matrix_table_clear_2a1b
// reason: mechanical translation of six SS:BP-stride zero stores preserving pushed registers

#include "recovered.hpp"

// label: matrix_table_clear_2a1b

extern "C" void CB_FAR cb_bloodprg_00963f_matrix_table_clear_2a1b(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->cx);
    m->push16(m->bp);
    m->bp = 0x2a1b;
    m->cx = 6;
    while (m->cx != 0) {
        m->write16(m->ss, m->bp, 0);
        cb_u16 before_add = m->bp;
        m->bp = (cb_u16)(m->bp + 0x18);
        m->set_add16_flags(before_add, 0x18, m->bp);
        m->cx = (cb_u16)(m->cx - 1);
    }
    m->bp = m->pop16();
    m->cx = m->pop16();
    m->ax = m->pop16();
    return;
}
