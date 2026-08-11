// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x006596
// assembly: re/assembly/bloodprg/seg_04da/func_006596_vm_op_a3_block.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_a3_block
// reason: mechanical translation of VM opcode 0xa3 conditional block handler

#include "recovered.hpp"

// label: vm_op_a3_block

extern "C" void CB_NEAR cb_bloodprg_006596_vm_op_a3_block(CbMachine* m)
{
    m->push16(m->di);
    cb_u8 block_gate = (cb_u8)(m->read8(m->gs, 0x67b2) & 1);
    m->set_logic8_flags(block_gate);
    if (block_gate != 0) {
        m->ax = 0;
        m->set_logic16_flags(m->ax);
        m->call_near(0x6293);
        m->di = m->pop16();
        return;
    }
    cb_set_lo8(m->dx, 0);
    m->set_logic8_flags(0);
    cb_set_lo8(m->ax, m->read8(m->ds, m->si));
    cb_u8 al = cb_lo8(m->ax);
    cb_u8 cmp_opcode = (cb_u8)(al - 0xa1);
    m->set_sub8_flags(al, 0xa1, cmp_opcode);
    if (cmp_opcode == 0) {
        cb_u16 before_inc_si = m->si;
        m->si = (cb_u16)(m->si + 1);
        m->set_inc16_flags(before_inc_si, m->si);
        cb_u8 dl_before = cb_lo8(m->dx);
        cb_u8 dl_after = (cb_u8)(dl_before + 1);
        cb_set_lo8(m->dx, dl_after);
        m->set_inc8_flags(dl_before, dl_after);
    }
    m->bp = 0x6762;
    m->ax = m->read16(m->ds, m->si);
    cb_advance_u16(m->si, 2, m->df);
    cb_u8 state_gate = (cb_u8)(m->read8(m->gs, 0x67b1) & 2);
    m->set_logic8_flags(state_gate);
    if (state_gate != 0) {
        m->bp = 0x6764;
    }
    cb_u16 field = m->read16(m->ss, m->bp);
    m->set_logic16_flags(field);
    if (field == 0) {
        m->call_near(0x6462);
        m->di = m->pop16();
        return;
    }
    cb_u16 cmp_value = (cb_u16)(m->ax - field);
    m->set_sub16_flags(m->ax, field, cmp_value);
    cb_u8 dl = cb_lo8(m->dx);
    if (cmp_value != 0) {
        m->set_logic8_flags(dl);
        if (dl == 0) {
            m->call_near(0x6462);
        }
        m->di = m->pop16();
        return;
    }
    m->set_logic8_flags(dl);
    if (dl != 0) {
        m->call_near(0x6462);
    }
    m->di = m->pop16();
    return;
}
