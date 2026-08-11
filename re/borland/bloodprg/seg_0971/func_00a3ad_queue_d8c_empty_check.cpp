// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a3ad
// assembly: re/assembly/bloodprg/seg_0971/func_00a3ad_queue_d8c_empty_check.asm
// provenance: recursive_graph
// status: translated_queue_d8c_empty_check
// reason: mechanical translation of queue head/tail/capacity comparisons

#include "recovered.hpp"

// label: queue_d8c_empty_check

extern "C" void CB_NEAR cb_bloodprg_00a3ad_queue_d8c_empty_check(CbMachine* m)
{
    m->ax = m->read16(m->ds, 0x0d8c);
    m->bx = m->read16(m->ds, 0x0d90);
    cb_u16 cmp_head_tail = (cb_u16)(m->ax - m->bx);
    m->set_sub16_flags(m->ax, m->bx, cmp_head_tail);
    if (m->ax < m->bx) {
        cb_u16 before_add = m->ax;
        m->ax = (cb_u16)(m->ax + m->cx);
        m->set_add16_flags(before_add, m->cx, m->ax);
        before_add = m->ax;
        m->ax = (cb_u16)(m->ax + 0x12);
        m->set_add16_flags(before_add, 0x12, m->ax);
        cb_u16 cmp_tail_limit = (cb_u16)(m->bx - m->ax);
        m->set_sub16_flags(m->bx, m->ax, cmp_tail_limit);
        if (m->bx < m->ax) {
            return;
        }
    }
    m->ax = m->read16(m->ds, 0x0d9a);
    cb_u16 before_add = m->ax;
    m->ax = (cb_u16)(m->ax + 0x0a);
    m->set_add16_flags(before_add, 0x0a, m->ax);
    before_add = m->ax;
    m->ax = (cb_u16)(m->ax + m->cx);
    m->set_add16_flags(before_add, m->cx, m->ax);
    if (m->cf) {
        return;
    }
    cb_u16 capacity = m->read16(m->ds, 0x0d98);
    cb_u16 cmp_capacity = (cb_u16)(capacity - m->ax);
    m->set_sub16_flags(capacity, m->ax, cmp_capacity);
    return;
}
