// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x006572
// assembly: re/assembly/bloodprg/seg_04da/func_006572_vm_op_a1_pop.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_a1_pop
// reason: mechanical translation of VM stack pop handler

#include "recovered.hpp"

// label: vm_op_a1_pop

extern "C" void CB_NEAR cb_bloodprg_006572_vm_op_a1_pop(CbMachine* m)
{
    m->write8(m->gs, 0x67ad, 0);
    m->ax = m->read16(m->gs, 0x6884);
    cb_u16 cmp_result = (cb_u16)(m->ax - 2);
    m->set_sub16_flags(m->ax, 2, cmp_result);
    if (cmp_result == 0) {
        return;
    }
    cb_u16 stack_ptr = m->read16(m->gs, 0x6884);
    cb_u16 result = (cb_u16)(stack_ptr - 2);
    m->write16(m->gs, 0x6884, result);
    m->set_sub16_flags(stack_ptr, 2, result);
    return;
}
