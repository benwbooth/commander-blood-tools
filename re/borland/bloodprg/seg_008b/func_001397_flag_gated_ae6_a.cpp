// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x001397
// assembly: re/assembly/bloodprg/seg_008b/func_001397_flag_gated_ae6_a.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_flag_gated_ae6_a
// reason: mechanical translation of MSCDEX drive-status gate preserving saved registers

#include "recovered.hpp"

// label: flag_gated_ae6_a

extern "C" void CB_FAR cb_bloodprg_001397_flag_gated_ae6_a(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->es);
    m->push16(m->bx);
    m->push16(m->cx);
    cb_u8 test_result = (cb_u8)(m->read8(m->gs, 0x0ae6) & 1);
    m->set_logic8_flags(test_result);
    if (test_result != 0) {
        m->ax = m->gs;
        m->es = m->ax;
        m->bx = 0x0b72;
        m->write8(m->es, m->bx, 0x0d);
        m->write8(m->es, (cb_u16)(m->bx + 2), 0x85);
        m->ax = 0x1510;
        m->cx = 0;
        m->set_logic16_flags(m->cx);
        cb_set_lo8(m->cx, m->read8(m->gs, 0x01b9));
        m->interrupt(0x2f);
    }
    m->cx = m->pop16();
    m->bx = m->pop16();
    m->es = m->pop16();
    m->ax = m->pop16();
    return;
}
