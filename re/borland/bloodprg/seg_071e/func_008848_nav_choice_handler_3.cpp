// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x008848
// assembly: re/assembly/bloodprg/seg_071e/func_008848_nav_choice_handler_3.asm
// provenance: static_dispatch_table_target
// status: translated_nav_choice_handler_3
// reason: mechanical translation of phase-bit guarded navigation state stores plus radio far call

#include "recovered.hpp"

// label: nav_choice_handler_3

extern "C" void CB_NEAR cb_bloodprg_008848_nav_choice_handler_3(CbMachine* m)
{
    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x2565) & 1);
    m->set_logic8_flags(test_result);
    if (test_result != 0) {
        m->ax = m->read16(m->ds, 0x6756);
        m->write16(m->ds, 0x676a, m->ax);
        m->write16(m->ds, 0x6768, 0x00c3);
        m->write8(m->ds, 0x2565, 0);
        m->si = 0x0d16;
        m->ax = 1;
        m->call_far(0x0b1b, 0x0855);
    }
    return;
}
