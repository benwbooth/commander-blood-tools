// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00763e
// assembly: re/assembly/bloodprg/seg_04da/func_00763e_routine.asm
// provenance: static_dispatch_table_target
// status: translated_byte_parser_string_d09
// reason: mechanical translation of printable-byte copy to ES:0x0d09 plus optional radio call

#include "recovered.hpp"

extern "C" void CB_NEAR cb_bloodprg_00763e_routine(CbMachine* m)
{
    m->di = 0x0d09;
    for (;;) {
        cb_set_lo8(m->ax, m->read8(m->ds, m->si));
        cb_advance_u16(m->si, 1, m->df);
        cb_u8 al = cb_lo8(m->ax);
        m->set_logic8_flags(al);
        if ((al & 0x80u) != 0) {
            break;
        }
        cb_u8 cmp_space = (cb_u8)(al - 0x20);
        m->set_sub8_flags(al, 0x20, cmp_space);
        if (al < 0x20) {
            break;
        }
        m->write8(m->es, m->di, al);
        cb_advance_u16(m->di, 1, m->df);
    }
    cb_u16 before_dec = m->si;
    m->si = (cb_u16)(m->si - 1);
    m->set_dec16_flags(before_dec, m->si);
    m->write8(m->es, m->di, 0);
    cb_u16 gate = (cb_u16)(m->read16(m->gs, 0x2793) & 1);
    m->set_logic16_flags(gate);
    if (gate == 0) {
        m->push16(m->ds);
        m->push16(m->si);
        m->ax = m->gs;
        m->ds = m->ax;
        m->si = 0x0d06;
        m->ax = 1;
        m->call_far(0x0b1b, 0x0855);
        m->si = m->pop16();
        m->ds = m->pop16();
    }
    return;
}
