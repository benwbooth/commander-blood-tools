// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00604e
// assembly: re/assembly/bloodprg/seg_04da/func_00604e_active_object_list_build.asm
// provenance: recursive_graph
// status: translated_active_object_list_build
// reason: mechanical translation of active object list builder from the GS lookup table

#include "recovered.hpp"

// label: active_object_list_build

extern "C" void CB_NEAR cb_bloodprg_00604e_active_object_list_build(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->bx);
    m->push16(m->ds);
    m->push16(m->si);
    m->push16(m->es);
    m->push16(m->di);
    m->push16(m->fs);
    m->ax = m->gs;
    m->es = m->ax;
    m->di = 0x6a16;
    m->si = m->read16(m->gs, 0x672c);
    m->ds = m->read16(m->gs, 0x672e);
    m->bx = m->read16(m->gs, 0x6724);
    m->fs = m->read16(m->gs, 0x6726);
    for (;;) {
        m->ax = m->read16(m->ds, (cb_u16)(m->si + 0x12));
        cb_u16 cmp_result = (cb_u16)(m->ax - 1);
        m->set_sub16_flags(m->ax, 1, cmp_result);
        if (cmp_result != 0) {
            break;
        }
        m->bx = m->read16(m->ds, (cb_u16)(m->si + 0x10));
        cb_u8 active = (cb_u8)(m->read8(m->fs, (cb_u16)(m->bx + 2)) & 2);
        m->set_logic8_flags(active);
        if (active != 0) {
            m->ax = m->bx;
            m->write16(m->es, m->di, m->ax);
            cb_advance_u16(m->di, 2, m->df);
        }
        cb_u16 before_add = m->si;
        m->si = (cb_u16)(m->si + 0x14);
        m->set_add16_flags(before_add, 0x14, m->si);
    }
    m->ax = 0xffff;
    m->write16(m->es, m->di, m->ax);
    cb_advance_u16(m->di, 2, m->df);
    m->fs = m->pop16();
    m->di = m->pop16();
    m->es = m->pop16();
    m->si = m->pop16();
    m->ds = m->pop16();
    m->bx = m->pop16();
    m->ax = m->pop16();
    return;
}
