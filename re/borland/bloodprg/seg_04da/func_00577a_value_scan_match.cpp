// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00577a
// assembly: re/assembly/bloodprg/seg_04da/func_00577a_value_scan_match.asm
// provenance: recursive_graph
// status: translated_value_scan_match
// reason: mechanical translation of linked value scan preserving SI

#include "recovered.hpp"

// label: value_scan_match

extern "C" void CB_NEAR cb_bloodprg_00577a_value_scan_match(CbMachine* m)
{
    cb_u16 saved_si = m->si;
    m->bx = m->ax;
    for (;;) {
        m->ax = m->read16(m->ds, m->si);
        cb_advance_u16(m->si, 2, m->df);
        cb_u16 cmp_result = (cb_u16)(m->ax - m->bx);
        m->set_sub16_flags(m->ax, m->bx, cmp_result);
        if (cmp_result == 0) {
            cb_u16 before_add = m->si;
            m->si = (cb_u16)(m->si + 2);
            m->set_add16_flags(before_add, 2, m->si);
            m->ax = m->si;
            m->si = saved_si;
            return;
        }
        m->si = m->read16(m->ds, m->si);
        m->set_logic16_flags(m->si);
        if (m->si == 0) {
            m->ax = m->si;
            m->si = saved_si;
            return;
        }
    }
}
