// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x006293
// assembly: re/assembly/bloodprg/seg_04da/func_006293_vm_token_special.asm
// provenance: recursive_graph
// status: translated_vm_token_special
// reason: mechanical translation of VM token stream scanner

#include "recovered.hpp"

// label: vm_token_special

extern "C" void CB_NEAR cb_bloodprg_006293_vm_token_special(CbMachine* m)
{
    for (;;) {
        cb_u16 current = m->read16(m->ds, m->si);
        cb_u16 cmp_word = (cb_u16)(m->ax - current);
        m->set_sub16_flags(m->ax, current, cmp_word);
        if (cmp_word == 0) {
            cb_u16 before_add = m->si;
            m->si = (cb_u16)(m->si + 2);
            m->set_add16_flags(before_add, 2, m->si);
            cb_u8 right = m->read8(m->ds, m->si);
            cb_u8 cmp_byte = (cb_u8)(cb_lo8(m->ax) - right);
            m->set_sub8_flags(cb_lo8(m->ax), right, cmp_byte);
            if (cmp_byte == 0) {
                cb_u16 before_inc = m->si;
                m->si = (cb_u16)(m->si + 1);
                m->set_inc16_flags(before_inc, m->si);
            }
            return;
        }
        m->si = (cb_u16)(m->si + 1);
    }
}
