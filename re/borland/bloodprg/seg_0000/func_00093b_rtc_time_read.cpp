// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00093b
// assembly: re/assembly/bloodprg/seg_0000/func_00093b_rtc_time_read.asm
// provenance: relocation_proven_far_transfer_target
// status: translated_rtc_time_read
// reason: mechanical translation of BIOS RTC time read plus BCD conversion call

#include "recovered.hpp"

// label: rtc_time_read

extern "C" void CB_FAR cb_bloodprg_00093b_rtc_time_read(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->cx);
    m->push16(m->dx);
    cb_set_hi8(m->ax, 2);
    m->interrupt(0x1a);
    cb_set_lo8(m->ax, cb_hi8(m->cx));
    m->call_near(0x0986);
    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);
    m->write16(m->gs, 0x0aa6, m->ax);
    m->dx = m->pop16();
    m->cx = m->pop16();
    m->ax = m->pop16();
    return;
}
