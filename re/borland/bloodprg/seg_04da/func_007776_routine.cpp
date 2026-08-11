// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x007776
// assembly: re/assembly/bloodprg/seg_04da/func_007776_routine.asm
// provenance: static_dispatch_table_target
// status: translated_prefixed_string_append
// reason: mechanical translation of MOVSW-prefixed string append through GS cursor

#include "recovered.hpp"

extern "C" void CB_NEAR cb_bloodprg_007776_routine(CbMachine* m)
{
    m->di = m->read16(m->gs, 0x0f18);
    cb_u16 prefix = m->read16(m->ds, m->si);
    m->write16(m->es, m->di, prefix);
    cb_advance_u16(m->si, 2, m->df);
    cb_advance_u16(m->di, 2, m->df);
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
    m->write16(m->gs, 0x0f18, m->di);
    return;
}
