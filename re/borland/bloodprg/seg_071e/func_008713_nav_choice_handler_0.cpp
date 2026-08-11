// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x008713
// assembly: re/assembly/bloodprg/seg_071e/func_008713_nav_choice_handler_0.asm
// provenance: static_dispatch_table_target
// status: translated_nav_choice_handler_0
// reason: mechanical translation of phase-bit guarded navigation state stores

#include "recovered.hpp"

// label: nav_choice_handler_0

extern "C" void CB_NEAR cb_bloodprg_008713_nav_choice_handler_0(CbMachine* m)
{
    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x2565) & 1);
    m->set_logic8_flags(test_result);
    if (test_result != 0) {
        m->ax = m->read16(m->ds, 0x6754);
        m->write16(m->ds, 0x676a, m->ax);
        m->write16(m->ds, 0x6768, 0x00c3);
        m->write8(m->ds, 0x2565, 0);
    }
    return;
}
