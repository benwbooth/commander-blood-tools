// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x000d4a
// assembly: re/assembly/bloodprg/seg_0000/func_000d4a_mouse_set_hrange.asm
// provenance: recursive_graph
// status: translated_mouse_set_hrange
// reason: mechanical translation of mouse int 33h horizontal/vertical range setup

#include "recovered.hpp"

// label: mouse_set_hrange

extern "C" void CB_FAR cb_bloodprg_000d4a_mouse_set_hrange(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->bx);
    m->push16(m->cx);
    m->push16(m->dx);
    m->cx = m->ax;
    m->dx = m->bx;
    m->ax = 7;
    m->interrupt(0x33);
    m->dx = m->pop16();
    m->cx = m->pop16();
    m->ax = 8;
    m->interrupt(0x33);
    m->bx = m->pop16();
    m->ax = m->pop16();
    return;
}
