// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00420d
// assembly: re/assembly/bloodprg/seg_0299/func_00420d_sprite_slot_position_update.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_sprite_slot_position_update
// reason: mechanical translation of active sprite-slot draw-position update and dirty flag

#include "recovered.hpp"

// label: sprite_slot_position_update

extern "C" void CB_FAR cb_bloodprg_00420d_sprite_slot_position_update(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->bx);
    m->push16(m->dx);
    m->ax = (cb_u16)(m->ax << 5);
    m->dx = m->bx;
    m->bx = 0x6212;
    cb_u16 before_add = m->bx;
    m->bx = (cb_u16)(m->bx + m->ax);
    m->set_add16_flags(before_add, m->ax, m->bx);
    m->ax = m->read16(m->gs, m->bx);
    cb_u8 al = cb_lo8(m->ax);
    cb_u8 active = (cb_u8)(al & 0x81u);
    m->set_logic8_flags(active);
    if (active != 0) {
        cb_u16 old_x = m->read16(m->gs, (cb_u16)(m->bx + 8));
        cb_u16 cmp_x = (cb_u16)(m->dx - old_x);
        m->set_sub16_flags(m->dx, old_x, cmp_x);
        if (cmp_x != 0) {
            al = (cb_u8)(al | 2);
            m->set_logic8_flags(al);
            cb_set_lo8(m->ax, al);
            m->write16(m->gs, (cb_u16)(m->bx + 8), m->dx);
        }
        cb_u16 old_y = m->read16(m->gs, (cb_u16)(m->bx + 0x0a));
        cb_u16 cmp_y = (cb_u16)(m->cx - old_y);
        m->set_sub16_flags(m->cx, old_y, cmp_y);
        if (cmp_y != 0) {
            al = (cb_u8)(al | 2);
            m->set_logic8_flags(al);
            cb_set_lo8(m->ax, al);
            m->write16(m->gs, (cb_u16)(m->bx + 0x0a), m->cx);
        }
    }
    m->write16(m->gs, m->bx, m->ax);
    m->dx = m->pop16();
    m->bx = m->pop16();
    m->ax = m->pop16();
    return;
}
