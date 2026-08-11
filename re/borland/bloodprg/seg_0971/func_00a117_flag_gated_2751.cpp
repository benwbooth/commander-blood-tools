// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a117
// assembly: re/assembly/bloodprg/seg_0971/func_00a117_flag_gated_2751.asm
// provenance: recursive_graph
// status: translated_flag_gated_2751_copy
// reason: mechanical translation of GS:0x2751-gated 0x60 dword copy

#include "recovered.hpp"

// label: flag_gated_2751

extern "C" void CB_NEAR cb_bloodprg_00a117_flag_gated_2751(CbMachine* m)
{
    m->push16(m->ds);
    m->push16(m->si);
    cb_u8 test_result = (cb_u8)(m->read8(m->gs, 0x2751) & 1);
    m->set_logic8_flags(test_result);
    if (test_result == 0) {
        m->cx = m->es;
        m->ds = m->cx;
        m->si = 0x5251;
        m->di = 0x5851;
        m->cx = 0x0060;
        while (m->cx != 0) {
            cb_u32 value = m->read32(m->ds, m->si);
            m->write32(m->es, m->di, value);
            cb_advance_u16(m->si, 4, m->df);
            cb_advance_u16(m->di, 4, m->df);
            m->cx = (cb_u16)(m->cx - 1);
        }
    }
    m->si = m->pop16();
    m->ds = m->pop16();
    return;
}
