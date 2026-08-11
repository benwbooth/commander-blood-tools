// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0065eb
// assembly: re/assembly/bloodprg/seg_04da/func_0065eb_vm_op_a5_cond_state_array.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_a5_cond_state_array
// reason: mechanical translation of VM opcode 0xa5 state-array conditional branch/store

#include "recovered.hpp"

// label: vm_op_a5_cond_state_array

extern "C" void CB_NEAR cb_bloodprg_0065eb_vm_op_a5_cond_state_array(CbMachine* m)
{
    cb_set_lo8(m->ax, m->read8(m->ds, m->si));
    cb_advance_u16(m->si, 1, m->df);
    m->ax = (cb_u16)(cb_i16)(cb_i8)cb_lo8(m->ax);
    cb_u16 before_add = m->ax;
    m->ax = (cb_u16)(m->ax + m->ax);
    m->set_add16_flags(before_add, before_add, m->ax);
    m->bp = m->ax;
    cb_u8 gate = (cb_u8)(m->read8(m->gs, 0x67ad) & 1);
    m->set_logic8_flags(gate);
    if (gate != 0) {
        cb_u16 state = m->read16(m->ss, (cb_u16)(m->bp + 0x6ade));
        m->set_logic16_flags(state);
        if (state != 0) {
            m->call_near(0x6462);
        }
        return;
    }
    m->ax = m->read16(m->ds, m->si);
    cb_advance_u16(m->si, 2, m->df);
    m->write16(m->ss, (cb_u16)(m->bp + 0x6ade), m->ax);
    return;
}
