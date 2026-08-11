// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a3d0
// assembly: re/assembly/bloodprg/seg_0971/func_00a3d0_queue_d8c_consume.asm
// provenance: recursive_graph
// status: translated_queue_d8c_consume
// reason: mechanical translation of variable-length queue record consume and bounds update

#include "recovered.hpp"

// label: queue_d8c_consume

extern "C" void CB_NEAR cb_bloodprg_00a3d0_queue_d8c_consume(CbMachine* m)
{
    m->si = m->read16(m->ds, 0x0d90);
    m->es = m->read16(m->ds, 0x0d92);
    m->ax = m->read16(m->es, m->si);
    cb_advance_u16(m->si, 2, m->df);
    cb_u16 count = m->read16(m->ds, 0x0d9a);
    cb_u16 count_result = (cb_u16)(count - m->ax);
    m->write16(m->ds, 0x0d9a, count_result);
    m->set_sub16_flags(count, m->ax, count_result);
    cb_u16 before_add = m->si;
    m->si = (cb_u16)(m->si + m->ax);
    m->set_add16_flags(before_add, m->ax, m->si);
    if (!m->cf) {
        cb_u16 limit = m->read16(m->ds, 0x5233);
        cb_u16 cmp_limit = (cb_u16)(m->si - limit);
        m->set_sub16_flags(m->si, limit, cmp_limit);
        if (m->si <= limit) {
            cb_u16 tail = m->read16(m->ds, 0x0d90);
            cb_u16 tail_result = (cb_u16)(tail + m->ax);
            m->write16(m->ds, 0x0d90, tail_result);
            m->set_add16_flags(tail, m->ax, tail_result);
            cb_u16 tick = m->read16(m->ds, 0x131c);
            cb_u16 tick_result = (cb_u16)(tick + 1);
            m->write16(m->ds, 0x131c, tick_result);
            m->set_inc16_flags(tick, tick_result);
            m->ax = m->read16(m->ds, 0x0d60);
            cb_u16 before_inc = m->ax;
            m->ax = (cb_u16)(m->ax + 1);
            m->set_inc16_flags(before_inc, m->ax);
            cb_u16 upper = m->read16(m->ds, 0x0d64);
            cb_u16 cmp_upper = (cb_u16)(m->ax - upper);
            m->set_sub16_flags(m->ax, upper, cmp_upper);
            if (m->ax > upper) {
                m->ax = 1;
                m->write16(m->ds, 0x0d64, 0xffff);
            }
            m->write16(m->ds, 0x0d60, m->ax);
            return;
        }
    }
    cb_u16 before_sub = m->ax;
    m->ax = (cb_u16)(m->ax - 2);
    m->set_sub16_flags(before_sub, 2, m->ax);
    m->write16(m->ds, 0x0d90, m->ax);
    m->ax = 0;
    m->set_logic16_flags(m->ax);
    cb_u16 tail = m->read16(m->ds, 0x0d90);
    cb_u16 tail_result = (cb_u16)(tail + m->ax);
    m->write16(m->ds, 0x0d90, tail_result);
    m->set_add16_flags(tail, m->ax, tail_result);
    cb_u16 tick = m->read16(m->ds, 0x131c);
    cb_u16 tick_result = (cb_u16)(tick + 1);
    m->write16(m->ds, 0x131c, tick_result);
    m->set_inc16_flags(tick, tick_result);
    m->ax = m->read16(m->ds, 0x0d60);
    cb_u16 before_inc = m->ax;
    m->ax = (cb_u16)(m->ax + 1);
    m->set_inc16_flags(before_inc, m->ax);
    cb_u16 upper = m->read16(m->ds, 0x0d64);
    cb_u16 cmp_upper = (cb_u16)(m->ax - upper);
    m->set_sub16_flags(m->ax, upper, cmp_upper);
    if (m->ax > upper) {
        m->ax = 1;
        m->write16(m->ds, 0x0d64, 0xffff);
    }
    m->write16(m->ds, 0x0d60, m->ax);
    return;
}
