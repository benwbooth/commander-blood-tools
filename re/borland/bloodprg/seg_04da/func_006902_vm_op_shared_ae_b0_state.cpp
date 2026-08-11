// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x006902
// assembly: re/assembly/bloodprg/seg_04da/func_006902_vm_op_shared_ae_b0_state.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_shared_ae_b0_state
// reason: mechanical translation of shared VM opcode 0xae/0xb0 state-table handler

#include "recovered.hpp"

// label: vm_op_shared_ae_b0_state

extern "C" void CB_NEAR cb_bloodprg_006902_vm_op_shared_ae_b0_state(CbMachine* m)
{
    m->push16(m->di);
    m->di = m->read16(m->gs, 0x6724);
    m->es = m->read16(m->gs, 0x6726);
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
    m->ax = m->read16(m->ds, m->si);
    cb_advance_u16(m->si, 2, m->df);
    m->bx = m->ax;
    m->ax = m->read16(m->ds, m->si);
    cb_advance_u16(m->si, 2, m->df);
    cb_u8 read_gate = (cb_u8)(m->read8(m->gs, 0x67ad) & 1);
    m->set_logic8_flags(read_gate);
    cb_u16 target = (cb_u16)(m->bx + m->di);
    cb_u8 dl = cb_lo8(m->dx);
    if (read_gate != 0) {
        m->ax = (cb_u16)(m->ax & m->read16(m->es, target));
        m->set_logic16_flags(m->ax);
        if (m->ax != 0) {
            m->set_logic8_flags(dl);
            if (dl != 0) {
                m->call_near(0x6462);
            }
        } else {
            m->set_logic8_flags(dl);
            if (dl == 0) {
                m->call_near(0x6462);
            }
        }
        m->di = m->pop16();
        return;
    }
    m->set_logic8_flags(dl);
    cb_u16 existing = m->read16(m->es, target);
    if (dl == 0) {
        cb_u16 result = (cb_u16)(existing | m->ax);
        m->write16(m->es, target, result);
        m->set_logic16_flags(result);
    } else {
        m->ax = (cb_u16)(~m->ax);
        cb_u16 result = (cb_u16)(existing & m->ax);
        m->write16(m->es, target, result);
        m->set_logic16_flags(result);
    }
    m->di = m->pop16();
    return;
}
