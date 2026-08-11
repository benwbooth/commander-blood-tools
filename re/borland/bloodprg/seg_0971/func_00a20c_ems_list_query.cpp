// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a20c
// assembly: re/assembly/bloodprg/seg_0971/func_00a20c_ems_list_query.asm
// provenance: recursive_graph
// status: translated_ems_list_query
// reason: mechanical translation of EMS list query/count gate and callback

#include "recovered.hpp"

// label: ems_list_query

extern "C" void CB_NEAR cb_bloodprg_00a20c_ems_list_query(CbMachine* m)
{
    cb_u16 active = m->read16(m->ds, 0x0d96);
    m->set_sub16_flags(active, 0, active);
    if (active > 0) {
        return;
    }
    m->cx = m->read16(m->ds, 0x0d9a);
    m->cf = 1;
    if (m->cx == 0) {
        return;
    }
    m->si = m->read16(m->ds, 0x0d90);
    m->es = m->read16(m->ds, 0x0d92);
    m->ax = m->read16(m->es, m->si);
    cb_advance_u16(m->si, 2, m->df);
    cb_u16 marker = m->read16(m->es, m->si);
    cb_u16 cmp_marker = (cb_u16)(marker - 0x6d6d);
    m->set_sub16_flags(marker, 0x6d6d, cmp_marker);
    if (cmp_marker != 0) {
        cb_u16 cmp_count = (cb_u16)(m->cx - m->ax);
        m->set_sub16_flags(m->cx, m->ax, cmp_count);
        if (m->cx < m->ax) {
            return;
        }
    }
    m->bp = m->read16(m->ds, 0x0abe);
    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x0d76) & 0x40u);
    m->set_logic8_flags(test_result);
    if (test_result != 0) {
        m->bp = m->read16(m->ds, 0x0da8);
    }
    m->call_near(0xa552);
    m->ax = 0;
    m->set_logic16_flags(m->ax);
    return;
}
