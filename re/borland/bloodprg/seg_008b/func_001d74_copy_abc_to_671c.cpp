// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x001d74
// assembly: re/assembly/bloodprg/seg_008b/func_001d74_copy_abc_to_671c.asm
// provenance: recursive_graph
// status: translated_copy_abc_to_671c
// reason: mechanical translation of GS far-pointer record copy loop

#include "recovered.hpp"

// label: copy_abc_to_671c

extern "C" void CB_NEAR cb_bloodprg_001d74_copy_abc_to_671c(CbMachine* m)
{
    m->push16(m->ds);
    m->push16(m->si);
    m->push16(m->es);
    m->push16(m->di);
    m->push16(m->cx);
    m->cx = m->ax;
    m->si = m->read16(m->gs, 0x0abc);
    m->ds = m->read16(m->gs, 0x0abe);
    m->di = m->read16(m->gs, 0x671c);
    m->es = m->read16(m->gs, 0x671e);
    for (;;) {
        m->ax = m->read16(m->ds, m->si);
        cb_advance_u16(m->si, 2, m->df);
        m->di = m->ax;
        cb_u8 value = m->read8(m->ds, m->si);
        m->write8(m->es, m->di, value);
        cb_advance_u16(m->si, 1, m->df);
        cb_advance_u16(m->di, 1, m->df);
        cb_u16 before_sub = m->cx;
        m->cx = (cb_u16)(m->cx - 3);
        m->set_sub16_flags(before_sub, 3, m->cx);
        if (m->cx == 0) {
            break;
        }
    }
    m->cx = m->pop16();
    m->di = m->pop16();
    m->es = m->pop16();
    m->si = m->pop16();
    m->ds = m->pop16();
    return;
}
