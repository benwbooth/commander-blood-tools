// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a38e
// assembly: re/assembly/bloodprg/seg_0971/func_00a38e_queue_d8c_wrap.asm
// provenance: recursive_graph
// status: translated_queue_d8c_wrap
// reason: mechanical translation of ring-buffer pointer wrap and count update

#include "recovered.hpp"

// label: queue_d8c_wrap

extern "C" void CB_NEAR cb_bloodprg_00a38e_queue_d8c_wrap(CbMachine* m)
{
    cb_u16 before_si = m->si;
    m->si = (cb_u16)(m->si + m->ax);
    m->set_add16_flags(before_si, m->ax, m->si);
    int wrap = m->cf;
    if (!wrap) {
        cb_u16 limit = m->read16(m->ds, 0x5233);
        cb_u16 cmp_result = (cb_u16)(m->si - limit);
        m->set_sub16_flags(m->si, limit, cmp_result);
        if (m->si <= limit) {
            wrap = 0;
        } else {
            wrap = 1;
        }
    }
    if (wrap) {
        m->cx = 0;
        m->set_logic16_flags(m->cx);
        cb_u16 old_head = m->read16(m->ds, 0x0d8c);
        m->write16(m->ds, 0x0d8c, m->cx);
        m->cx = old_head;
        m->write16(m->ds, 0x0d98, m->cx);
    }
    cb_u16 before_sub = m->ax;
    m->ax = (cb_u16)(m->ax - 2);
    m->set_sub16_flags(before_sub, 2, m->ax);
    m->write16(m->ds, 0x0da0, m->ax);
    cb_u16 count = m->read16(m->ds, 0x0d62);
    cb_u16 count_result = (cb_u16)(count + 1);
    m->write16(m->ds, 0x0d62, count_result);
    m->set_inc16_flags(count, count_result);
    return;
}
