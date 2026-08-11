// Commander Blood Borland C++ translation unit
// module: xdb_scrut
// overlay_offset: 0x001bfb
// assembly: re/assembly/xdb/scrut/method_table_103a/func_001bfb_routine.asm
// provenance: alien_method_table_103a_slot_13@0x4404
// status: translated_xdb_jump_or_init_method
// reason: mechanical translation of XDB method jump-or-initialize sequence

#include "recovered.hpp"

extern "C" void CB_NEAR cb_xdb_scrut_001bfb_routine(CbMachine* m)
{
    m->bx = m->read16(m->ds, (cb_u16)(m->di + 0x36));
    m->set_logic16_flags(m->bx);
    if (m->bx != 0) {
        m->jump_near(m->bx);
        return;
    }
    m->write16(m->ds, (cb_u16)(m->di + 0x36), 0x1c45);
    m->write16(m->ds, (cb_u16)(m->di + 0x38), 0);
    m->write16(m->ds, (cb_u16)(m->di + 0x3a), 0);
    return;
}
