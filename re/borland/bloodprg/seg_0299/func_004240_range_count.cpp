// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x004240
// assembly: re/assembly/bloodprg/seg_0299/func_004240_range_count.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_range_count
// reason: mechanical translation of inclusive range entity-flag update loop

#include "recovered.hpp"

// label: range_count

extern "C" void CB_FAR cb_bloodprg_004240_range_count(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->bx);
    m->push16(m->cx);
    m->push16(m->ds);
    m->push16(m->si);
    m->cx = m->bx;
    cb_u16 before_sub = m->cx;
    m->cx = (cb_u16)(m->cx - m->ax);
    m->set_sub16_flags(before_sub, m->ax, m->cx);
    cb_u16 before_inc = m->cx;
    m->cx = (cb_u16)(m->cx + 1);
    m->set_inc16_flags(before_inc, m->cx);
    m->bx = m->gs;
    m->ds = m->bx;
    m->si = 0x6212;
    m->ax = (cb_u16)(m->ax << 5);
    cb_u16 before_add = m->si;
    m->si = (cb_u16)(m->si + m->ax);
    m->set_add16_flags(before_add, m->ax, m->si);
    for (;;) {
        m->ax = m->read16(m->ds, m->si);
        cb_u8 al = cb_lo8(m->ax);
        m->set_logic8_flags(al);
        if ((al & 0x80u) != 0) {
            al = (cb_u8)(al & 0x7eu);
            m->set_logic8_flags(al);
            al = (cb_u8)(al | 2);
            m->set_logic8_flags(al);
            cb_set_lo8(m->ax, al);
            m->write16(m->ds, m->si, m->ax);
        }
        before_add = m->si;
        m->si = (cb_u16)(m->si + 0x20);
        m->set_add16_flags(before_add, 0x20, m->si);
        m->cx = (cb_u16)(m->cx - 1);
        if (m->cx == 0) {
            break;
        }
    }
    m->si = m->pop16();
    m->ds = m->pop16();
    m->cx = m->pop16();
    m->bx = m->pop16();
    m->ax = m->pop16();
    return;
}
