// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x000bff
// assembly: re/assembly/bloodprg/seg_0000/func_000bff_install_ctrl_break_handler.asm
// provenance: recursive_graph
// status: translated_install_ctrl_break_handler
// reason: mechanical translation of DOS int 21h vector setup preserving AX/DX/DS

#include "recovered.hpp"

// label: install_ctrl_break_handler

extern "C" void CB_FAR cb_bloodprg_000bff_install_ctrl_break_handler(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->dx);
    m->push16(m->ds);
    m->ax = m->cs;
    m->ds = m->ax;
    m->ax = 0x2523;
    m->dx = 0x0619;
    m->interrupt(0x21);
    cb_set_lo8(m->ax, 0x24);
    m->dx = 0x061a;
    m->interrupt(0x21);
    m->ds = m->pop16();
    m->dx = m->pop16();
    m->ax = m->pop16();
    return;
}
