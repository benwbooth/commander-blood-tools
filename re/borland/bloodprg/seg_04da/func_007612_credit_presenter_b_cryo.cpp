// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x007612
// assembly: re/assembly/bloodprg/seg_04da/func_007612_credit_presenter_b_cryo.asm
// provenance: static_dispatch_table_target
// status: translated_credit_presenter_b_cryo_copy
// reason: mechanical translation of NUL-terminated copy to ES:0x0e18 plus GS state stores

#include "recovered.hpp"

// label: credit_presenter_b_cryo

extern "C" void CB_NEAR cb_bloodprg_007612_credit_presenter_b_cryo(CbMachine* m)
{
    m->di = 0x0e18;
    for (;;) {
        cb_set_lo8(m->ax, m->read8(m->ds, m->si));
        cb_advance_u16(m->si, 1, m->df);
        m->write8(m->es, m->di, cb_lo8(m->ax));
        cb_advance_u16(m->di, 1, m->df);
        m->set_logic8_flags(cb_lo8(m->ax));
        if (cb_lo8(m->ax) == 0) {
            break;
        }
    }
    m->write8(m->gs, 0x5e64, 1);
    m->write16(m->gs, 0x5e58, 0);
    return;
}
