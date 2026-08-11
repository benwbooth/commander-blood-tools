// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00bd8d
// assembly: re/assembly/bloodprg/seg_0b1b/func_00bd8d_ems_page_offset_split.asm
// provenance: recursive_graph
// status: translated_ems_page_offset_split
// reason: mechanical translation of DOS seek/read EMS-page split helper

#include "recovered.hpp"

// label: ems_page_offset_split

extern "C" void CB_NEAR cb_bloodprg_00bd8d_ems_page_offset_split(CbMachine* m)
{
    m->push16(m->ds);
    m->push16(m->ax);
    m->push16(m->bx);
    m->push16(m->cx);
    m->push16(m->dx);
    m->cx = m->ax;
    m->cx = (cb_u16)(m->cx >> 2);
    m->ax = (cb_u16)(m->ax << 14);
    m->dx = m->ax;
    m->bx = m->read16(m->gs, 0x0c49);
    m->ax = 0x4200;
    m->interrupt(0x21);
    m->cx = 0x4000;
    cb_set_hi8(m->ax, 0x3f);
    m->push16(m->es);
    m->ds = m->pop16();
    m->dx = m->di;
    m->interrupt(0x21);
    m->dx = m->pop16();
    m->cx = m->pop16();
    m->bx = m->pop16();
    m->ax = m->pop16();
    m->ds = m->pop16();
    return;
}
