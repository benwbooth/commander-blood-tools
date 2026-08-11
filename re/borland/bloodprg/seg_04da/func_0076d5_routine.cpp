// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0076d5
// assembly: re/assembly/bloodprg/seg_04da/func_0076d5_routine.asm
// provenance: static_dispatch_table_target
// status: translated_control_byte_copy
// reason: mechanical translation of control-byte-terminated copy to ES:0x247a

#include "recovered.hpp"

extern "C" void CB_NEAR cb_bloodprg_0076d5_routine(CbMachine* m)
{
    m->di = 0x247a;
    for (;;) {
        cb_set_lo8(m->ax, m->read8(m->ds, m->si));
        cb_advance_u16(m->si, 1, m->df);
        cb_u8 value = cb_lo8(m->ax);
        m->set_logic8_flags(value);
        if ((value & 0x80u) != 0) {
            break;
        }
        cb_u8 cmp_result = (cb_u8)(value - 0x20);
        m->set_sub8_flags(value, 0x20, cmp_result);
        if (value < 0x20u) {
            break;
        }
        m->write8(m->es, m->di, value);
        cb_advance_u16(m->di, 1, m->df);
    }
    cb_u16 before_dec = m->si;
    m->si = (cb_u16)(m->si - 1);
    m->set_dec16_flags(before_dec, m->si);
    m->write8(m->es, m->di, 0);
    return;
}
