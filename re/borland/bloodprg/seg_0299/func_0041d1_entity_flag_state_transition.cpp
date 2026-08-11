// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0041d1
// assembly: re/assembly/bloodprg/seg_0299/func_0041d1_entity_flag_state_transition.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_entity_flag_state_transition
// reason: mechanical translation of GS entity flag state transition preserving AX/BX

#include "recovered.hpp"

// label: entity_flag_state_transition

extern "C" void CB_FAR cb_bloodprg_0041d1_entity_flag_state_transition(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->bx);
    m->ax = (cb_u16)(m->ax << 5);
    m->bx = 0x6212;
    cb_u16 before_add = m->bx;
    m->bx = (cb_u16)(m->bx + m->ax);
    m->set_add16_flags(before_add, m->ax, m->bx);
    m->ax = m->read16(m->gs, m->bx);
    cb_u8 al = cb_lo8(m->ax);
    m->set_logic8_flags(al);
    if ((al & 0x80u) != 0) {
        cb_u8 test_result = (cb_u8)(al & 1);
        m->set_logic8_flags(test_result);
        if (test_result != 0) {
            al = (cb_u8)(al & 0xfeu);
            m->set_logic8_flags(al);
            al = (cb_u8)(al | 2);
            m->set_logic8_flags(al);
            cb_set_lo8(m->ax, al);
        }
    }
    m->write16(m->gs, m->bx, m->ax);
    m->bx = m->pop16();
    m->ax = m->pop16();
    return;
}
