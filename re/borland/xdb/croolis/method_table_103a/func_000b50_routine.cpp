// Commander Blood Borland C++ translation unit
// module: xdb_croolis
// overlay_offset: 0x000b50
// assembly: re/assembly/xdb/croolis/method_table_103a/func_000b50_routine.asm
// provenance: alien_method_table_103a_slot_11@0x4340
// status: translated_xdb_actor_field_sub_0f
// reason: mechanical translation of XDB actor field subtract and optional CS slot update

#include "recovered.hpp"

extern "C" void CB_NEAR cb_xdb_croolis_000b50_routine(CbMachine* m)
{
    m->si = m->read16(m->ds, (cb_u16)(m->di + 0x16));
    cb_u16 before_add = m->si;
    m->si = (cb_u16)(m->si + 0x5e);
    m->set_add16_flags(before_add, 0x5e, m->si);
    cb_u16 field_addr = (cb_u16)(m->si + 0x52);
    cb_u16 field_value = m->read16(m->ds, field_addr);
    cb_u16 sub_result = (cb_u16)(field_value - 0x0f);
    m->write16(m->ds, field_addr, sub_result);
    m->set_sub16_flags(field_value, 0x0f, sub_result);
    m->write16(m->cs, 0x1b2e, m->si);
    return;
}
