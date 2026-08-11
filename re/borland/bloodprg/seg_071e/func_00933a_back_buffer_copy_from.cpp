// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00933a
// assembly: re/assembly/bloodprg/seg_071e/func_00933a_back_buffer_copy_from.asm
// provenance: recursive_graph
// status: translated_back_buffer_copy_from
// reason: mechanical translation of GS far-pointer row copy to back buffer

#include "recovered.hpp"

// label: back_buffer_copy_from

extern "C" void CB_NEAR cb_bloodprg_00933a_back_buffer_copy_from(CbMachine* m)
{
    m->push16(m->es);
    m->push16(m->di);
    m->push16(m->ds);
    m->push16(m->si);
    m->push16(m->cx);
    m->push16(m->ax);
    m->di = m->read16(m->gs, 0x5229);
    m->es = m->read16(m->gs, 0x522b);
    m->si = m->read16(m->gs, 0x0abc);
    m->ds = m->read16(m->gs, 0x0abe);
    m->ax = m->cx;
    cb_u8 old_ah = cb_hi8(m->ax);
    cb_set_hi8(m->ax, cb_lo8(m->ax));
    cb_set_lo8(m->ax, old_ah);
    m->cx = (cb_u16)(m->cx << 6);
    cb_u16 before_add = m->ax;
    m->ax = (cb_u16)(m->ax + m->cx);
    m->set_add16_flags(before_add, m->cx, m->ax);
    m->di = m->ax;
    before_add = m->di;
    m->di = (cb_u16)(m->di + m->bx);
    m->set_add16_flags(before_add, m->bx, m->di);
    m->si = m->di;
    m->cx = m->dx;
    while (m->cx != 0) {
        cb_u8 value = m->read8(m->ds, m->si);
        m->write8(m->es, m->di, value);
        cb_advance_u16(m->si, 1, m->df);
        cb_advance_u16(m->di, 1, m->df);
        m->cx = (cb_u16)(m->cx - 1);
    }
    m->ax = m->pop16();
    m->cx = m->pop16();
    m->si = m->pop16();
    m->ds = m->pop16();
    m->di = m->pop16();
    m->es = m->pop16();
    return;
}
