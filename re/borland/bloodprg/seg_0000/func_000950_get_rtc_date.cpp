// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x000950
// assembly: re/assembly/bloodprg/seg_0000/func_000950_get_rtc_date.asm
// provenance: relocation_proven_far_transfer_target
// status: translated_rtc_date_read
// reason: mechanical translation of BIOS RTC date read plus decoded GS stores

#include "recovered.hpp"

// label: get_rtc_date

extern "C" void CB_FAR cb_bloodprg_000950_get_rtc_date(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->cx);
    m->push16(m->dx);
    cb_set_hi8(m->ax, 4);
    m->interrupt(0x1a);
    cb_set_lo8(m->ax, cb_lo8(m->dx));
    m->call_near(0x0986);
    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);
    m->write16(m->gs, 0x0aa8, m->ax);
    cb_set_lo8(m->ax, cb_hi8(m->dx));
    m->call_near(0x0986);
    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);
    m->write16(m->gs, 0x0aaa, m->ax);
    cb_set_lo8(m->ax, cb_lo8(m->cx));
    m->call_near(0x0986);
    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);
    cb_u8 ch_value = cb_hi8(m->cx);
    cb_u8 cmp_ch = (cb_u8)(ch_value - 0x13);
    m->set_sub8_flags(ch_value, 0x13, cmp_ch);
    cb_u16 before_add = m->ax;
    if (cmp_ch == 0) {
        m->ax = (cb_u16)(m->ax + 0x076c);
        m->set_add16_flags(before_add, 0x076c, m->ax);
    } else {
        m->ax = (cb_u16)(m->ax + 0x07d0);
        m->set_add16_flags(before_add, 0x07d0, m->ax);
    }
    m->write16(m->gs, 0x0aac, m->ax);
    m->dx = m->pop16();
    m->cx = m->pop16();
    m->ax = m->pop16();
    return;
}
