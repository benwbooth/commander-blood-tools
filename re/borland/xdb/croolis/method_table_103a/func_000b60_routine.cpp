// Commander Blood Borland C++ translation unit
// module: xdb_croolis
// overlay_offset: 0x000b60
// assembly: re/assembly/xdb/croolis/method_table_103a/func_000b60_routine.asm
// provenance: alien_method_table_103a_slot_12@0x4342
// status: translated_xdb_add_cs99_if_nonnegative
// reason: mechanical translation of XDB CS:0x99 SAR/JS/add sequence

#include "recovered.hpp"

extern "C" void CB_NEAR cb_xdb_croolis_000b60_routine(CbMachine* m)
{
    m->si = m->read16(m->ds, (cb_u16)(m->di + 0x16));
    m->ax = m->read16(m->cs, 0x0099);
    cb_u16 before_sar = m->ax;
    m->ax = (cb_u16)((before_sar >> 1) | (before_sar & 0x8000u));
    m->set_sar16_flags(before_sar, 1, m->ax);
    if ((m->ax & 0x8000u) == 0) {
        cb_u16 field_addr = (cb_u16)(m->si + 0x00b0);
        cb_u16 field_value = m->read16(m->ds, field_addr);
        cb_u16 add_result = (cb_u16)(field_value + m->ax);
        m->write16(m->ds, field_addr, add_result);
        m->set_add16_flags(field_value, m->ax, add_result);
    }
    return;
}
