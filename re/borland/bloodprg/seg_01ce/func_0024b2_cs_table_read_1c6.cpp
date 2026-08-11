// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0024b2
// assembly: re/assembly/bloodprg/seg_01ce/func_0024b2_cs_table_read_1c6.asm
// provenance: relocation_proven_far_transfer_target
// status: translated_cs_table_read_1c6
// reason: mechanical translation of far decimal conversion helper using CS scratch table

#include "recovered.hpp"

// label: cs_table_read_1c6

extern "C" void CB_FAR cb_bloodprg_0024b2_cs_table_read_1c6(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->cx);
    m->push16(m->dx);
    m->push16(m->di);
    m->push16(m->ds);
    m->push16(m->si);
    m->dx = m->cs;
    m->ds = m->dx;
    m->si = 0x01c6;
    cb_u16 before_add = m->si;
    m->si = (cb_u16)(m->si + 0x0b);
    m->set_add16_flags(before_add, 0x0b, m->si);
    m->cx = 0x000a;
    m->set_logic16_flags(m->ax);
    if ((m->ax & 0x8000u) != 0) {
        m->write8(m->es, m->di, 0x2d);
        cb_u16 before_inc = m->di;
        m->di = (cb_u16)(m->di + 1);
        m->set_inc16_flags(before_inc, m->di);
        cb_u16 neg_input = m->ax;
        m->ax = (cb_u16)(0 - m->ax);
        m->set_sub16_flags(0, neg_input, m->ax);
    }
    for (;;) {
        cb_u16 before_dec = m->si;
        m->si = (cb_u16)(m->si - 1);
        m->set_dec16_flags(before_dec, m->si);
        m->dx = 0;
        m->set_logic16_flags(m->dx);
        cb_u32 dividend = m->ax;
        cb_u16 divisor = m->cx;
        m->ax = (cb_u16)(dividend / divisor);
        m->dx = (cb_u16)(dividend % divisor);
        before_add = m->dx;
        m->dx = (cb_u16)(m->dx + 0x30);
        m->set_add16_flags(before_add, 0x30, m->dx);
        m->write8(m->ds, m->si, cb_lo8(m->dx));
        m->set_logic16_flags(m->ax);
        if (m->ax == 0) {
            break;
        }
    }
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
    m->si = m->pop16();
    m->ds = m->pop16();
    m->di = m->pop16();
    m->dx = m->pop16();
    m->cx = m->pop16();
    m->ax = m->pop16();
    return;
}
