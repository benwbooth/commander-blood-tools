// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0064e5
// assembly: re/assembly/bloodprg/seg_04da/func_0064e5_vm_op_ca_compare_var.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_ca_compare_var
// reason: mechanical translation of VM opcode 0xca signed state compare and branch helper call

#include "recovered.hpp"

// label: vm_op_ca_compare_var

extern "C" void CB_NEAR cb_bloodprg_0064e5_vm_op_ca_compare_var(CbMachine* m)
{
    m->ax = m->read16(m->ds, m->si);
    cb_advance_u16(m->si, 2, m->df);
    cb_set_lo8(m->dx, cb_lo8(m->ax));
    m->ax = m->read16(m->ds, m->si);
    cb_advance_u16(m->si, 2, m->df);
    cb_u8 dl = cb_lo8(m->dx);
    cb_u8 cmp_tag = (cb_u8)(dl - 0xf1);
    m->set_sub8_flags(dl, 0xf1, cmp_tag);
    if (cmp_tag == 0) {
        cb_u16 state = m->read16(m->gs, 0x0aa6);
        cb_u16 cmp_value = (cb_u16)(m->ax - state);
        m->set_sub16_flags(m->ax, state, cmp_value);
        if ((cb_i16)m->ax > (cb_i16)state) {
            return;
        }
        m->call_near(0x6462);
        return;
    }
    cmp_tag = (cb_u8)(dl - 0xf2);
    m->set_sub8_flags(dl, 0xf2, cmp_tag);
    if (cmp_tag == 0) {
        cb_u16 state = m->read16(m->gs, 0x0aa6);
        cb_u16 cmp_value = (cb_u16)(m->ax - state);
        m->set_sub16_flags(m->ax, state, cmp_value);
        if ((cb_i16)m->ax < (cb_i16)state) {
            return;
        }
        m->call_near(0x6462);
        return;
    }
    cb_u16 state = m->read16(m->gs, 0x0aa6);
    cb_u16 cmp_value = (cb_u16)(m->ax - state);
    m->set_sub16_flags(m->ax, state, cmp_value);
    if (cmp_value == 0) {
        return;
    }
    m->call_near(0x6462);
    return;
}
