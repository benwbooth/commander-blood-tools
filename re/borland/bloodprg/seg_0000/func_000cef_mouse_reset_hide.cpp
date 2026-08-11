// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x000cef
// assembly: re/assembly/bloodprg/seg_0000/func_000cef_mouse_reset_hide.asm
// provenance: recursive_graph
// status: translated_mouse_reset_hide
// reason: mechanical translation of mouse int 33h reset/hide/mickey-ratio setup

#include "recovered.hpp"

// label: mouse_reset_hide

extern "C" void CB_FAR cb_bloodprg_000cef_mouse_reset_hide(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->bx);
    m->push16(m->cx);
    m->push16(m->dx);
    m->push16(m->es);
    m->ax = 0;
    m->set_logic16_flags(m->ax);
    m->interrupt(0x33);
    m->ax = 2;
    m->interrupt(0x33);
    m->cx = 0x000c;
    m->dx = 0x000c;
    m->ax = 0x000f;
    m->interrupt(0x33);
    m->es = m->pop16();
    m->dx = m->pop16();
    m->cx = m->pop16();
    m->bx = m->pop16();
    m->ax = m->pop16();
    return;
}
