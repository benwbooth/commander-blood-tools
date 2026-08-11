// Commander Blood Borland C++ translation unit
// module: xdb_amer
// overlay_offset: 0x001bea
// assembly: re/assembly/xdb/amer/method_table_103a/func_001bea_routine.asm
// provenance: alien_method_table_103a_slot_13@0x42d4
// status: translated_xdb_jump_or_init_method
// reason: mechanical translation of XDB method jump-or-initialize sequence

#include "recovered.hpp"

extern "C" void CB_NEAR cb_xdb_amer_001bea_routine(CbMachine* m)
{
    m->bx = m->read16(m->ds, (cb_u16)(m->di + 0x36));
    m->set_logic16_flags(m->bx);
    if (m->bx != 0) {
        m->jump_near(m->bx);
        return;
    }
    m->write16(m->ds, (cb_u16)(m->di + 0x36), 0x1c34);
    m->write16(m->ds, (cb_u16)(m->di + 0x38), 0);
    m->write16(m->ds, (cb_u16)(m->di + 0x3a), 0);
    return;
}
