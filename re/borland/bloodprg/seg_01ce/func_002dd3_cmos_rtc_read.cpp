// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x002dd3
// assembly: re/assembly/bloodprg/seg_01ce/func_002dd3_cmos_rtc_read.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_cmos_rtc_read
// reason: mechanical translation of CMOS port select/read and CS state store

#include "recovered.hpp"

// label: cmos_rtc_read

extern "C" void CB_FAR cb_bloodprg_002dd3_cmos_rtc_read(CbMachine* m)
{
    m->push16(m->ax);
    m->ax = 0;
    m->set_logic16_flags(m->ax);
    m->out8(0x0070, cb_lo8(m->ax));
    cb_set_lo8(m->ax, m->in8(0x0071));
    cb_set_hi8(m->ax, cb_lo8(m->ax));
    m->write16(m->cs, 0x0aee, m->ax);
    m->ax = m->pop16();
    return;
}
