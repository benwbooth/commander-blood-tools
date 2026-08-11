// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x009f53
// assembly: re/assembly/bloodprg/seg_0971/func_009f53_presentation_update_1fb2.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_presentation_update_1fb2
// reason: mechanical translation of presentation update gate and dirty-state stores

#include "recovered.hpp"

// label: presentation_update_1fb2

extern "C" void CB_FAR cb_bloodprg_009f53_presentation_update_1fb2(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->bx);
    m->push16(m->cx);
    cb_u8 gate = (cb_u8)(m->read8(m->ds, 0x1fb2) & 1);
    m->set_logic8_flags(gate);
    if (gate != 0) {
        m->call_near(0xa2dd);
        cb_u8 flag = (cb_u8)(m->read8(m->ds, 0x24f3) & 8);
        m->set_logic8_flags(flag);
        if (flag != 0) {
            m->write8(m->ds, 0x27d8, 1);
        }
        m->write16(m->ds, 0x6788, 0xffff);
        m->write8(m->ds, 0x1fb2, 0);
        cb_u8 value = (cb_u8)(m->read8(m->ds, 0x67aa) & 0xfdu);
        m->write8(m->ds, 0x67aa, value);
        m->set_logic8_flags(value);
    }
    m->cx = m->pop16();
    m->bx = m->pop16();
    m->ax = m->pop16();
    return;
}
