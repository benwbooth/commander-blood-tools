// Commander Blood Borland C++ translation unit
// module: xdb_amer
// overlay_offset: 0x001b8f
// assembly: re/assembly/xdb/amer/method_table_103a/func_001b8f_routine.asm
// provenance: alien_method_table_103a_slot_9@0x42cc
// status: translated_xdb_field_delta_sar4
// reason: mechanical translation of XDB slot SAR4 field-delta propagation loop

#include "recovered.hpp"

extern "C" void CB_NEAR cb_xdb_amer_001b8f_routine(CbMachine* m)
{
    m->push16(m->ds);
    m->si = m->read16(m->ds, (cb_u16)(m->di + 0x38));
    m->bx = m->read16(m->ds, (cb_u16)(m->di + 0x3a));
    m->ax = m->read16(m->ds, (cb_u16)(m->si + 0x36));
    cb_u16 before_add = m->si;
    m->si = (cb_u16)(m->si + 4);
    m->set_add16_flags(before_add, 4, m->si);
    m->si = (cb_u16)(m->si & 0x0ffcu);
    m->set_logic16_flags(m->si);
    m->write16(m->ds, (cb_u16)(m->di + 0x38), m->si);
    cb_u16 before_sar = m->ax;
    if ((before_sar & 0x8000u) != 0) {
        m->ax = (cb_u16)((before_sar >> 4) | 0xf000u);
    } else {
        m->ax = (cb_u16)(before_sar >> 4);
    }
    m->set_sar16_flags(before_sar, 4, m->ax);
    m->write16(m->ds, (cb_u16)(m->di + 0x3a), m->ax);
    cb_u16 before_sub = m->ax;
    m->ax = (cb_u16)(m->ax - m->bx);
    m->set_sub16_flags(before_sub, m->bx, m->ax);
    m->ds = m->read16(m->fs, 0x0002);
    m->si = m->read16(m->fs, (cb_u16)(m->di + 0x1c));
    m->cx = m->read16(m->fs, (cb_u16)(m->di + 0x20));
    for (;;) {
        cb_u16 field_value = m->read16(m->ds, m->si);
        cb_u16 field_result = (cb_u16)(field_value + m->ax);
        m->write16(m->ds, m->si, field_result);
        m->set_add16_flags(field_value, m->ax, field_result);
        before_add = m->si;
        m->si = (cb_u16)(m->si + 0x14);
        m->set_add16_flags(before_add, 0x14, m->si);
        m->cx = (cb_u16)(m->cx - 1);
        if (m->cx == 0) {
            break;
        }
    }
    m->ds = m->pop16();
    return;
}
