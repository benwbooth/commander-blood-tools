// Commander Blood Borland C++ translation unit
// module: xdb_manu3
// overlay_offset: 0x000181
// assembly: re/assembly/xdb/manu3/manu3_labeled/func_000181_manu3_anim_select.asm
// provenance: direct_call_from_0x0, direct_call_from_0x17c, label:manu3_anim_select, manu3 animation selector
// status: translated_manu3_anim_select
// reason: mechanical translation of MANU3 sequence-table selection and tail jump

#include "recovered.hpp"

// label: manu3_anim_select

extern "C" void CB_NEAR cb_xdb_manu3_000181_manu3_anim_select(CbMachine* m)
{
    m->bx = (cb_u16)(m->bx & 0x001f);
    m->set_logic16_flags(m->bx);
    cb_u16 before_add = m->bx;
    m->bx = (cb_u16)(m->bx + m->bx);
    m->set_add16_flags(before_add, before_add, m->bx);
    m->di = m->read16(m->ds, 0x2306);
    m->write16(m->ds, 0x102c, 0);
    cb_u16 addend = m->read16(m->ds, (cb_u16)(m->bx + m->di));
    before_add = m->di;
    m->di = (cb_u16)(m->di + addend);
    m->set_add16_flags(before_add, addend, m->di);
    m->write16(m->ds, 0x102e, m->di);
    m->bx = 0x1032;
    m->jump_near(0x01df);
    return;
}
