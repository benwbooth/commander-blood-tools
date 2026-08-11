// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x000d0e
// assembly: re/assembly/bloodprg/seg_0000/func_000d0e_poll_mouse.asm
// provenance: relocation_proven_far_transfer_target
// status: translated_poll_mouse
// reason: mechanical translation of mouse int 33h poll and movement-change stores

#include "recovered.hpp"

// label: poll_mouse

extern "C" void CB_FAR cb_bloodprg_000d0e_poll_mouse(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->bx);
    m->push16(m->cx);
    m->push16(m->dx);
    m->ax = 3;
    m->interrupt(0x33);
    m->write16(m->gs, 0x0a2a, m->cx);
    m->write16(m->gs, 0x0a2c, m->dx);
    m->write16(m->gs, 0x0a2e, m->bx);
    cb_u16 old_x = m->read16(m->gs, 0x0a38);
    cb_u16 cmp_x = (cb_u16)(m->cx - old_x);
    m->set_sub16_flags(m->cx, old_x, cmp_x);
    if (cmp_x != 0) {
        m->write16(m->gs, 0x0a38, m->cx);
        m->write16(m->gs, 0x0a3a, m->dx);
        m->write16(m->gs, 0x0b3b, 0);
    } else {
        cb_u16 old_y = m->read16(m->gs, 0x0a3a);
        cb_u16 cmp_y = (cb_u16)(m->dx - old_y);
        m->set_sub16_flags(m->dx, old_y, cmp_y);
        if (cmp_y != 0) {
            m->write16(m->gs, 0x0a38, m->cx);
            m->write16(m->gs, 0x0a3a, m->dx);
            m->write16(m->gs, 0x0b3b, 0);
        }
    }
    m->dx = m->pop16();
    m->cx = m->pop16();
    m->bx = m->pop16();
    m->ax = m->pop16();
    return;
}
