// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0064a0
// assembly: re/assembly/bloodprg/seg_04da/func_0064a0_vm_op_d0_cond_branch.asm
// provenance: static_dispatch_table_target
// status: translated_vm_flag_clear_branch
// reason: mechanical translation of TEST GS:0x252a,1 plus conditional call to VM branch helper

#include "recovered.hpp"

// label: vm_op_d0_cond_branch

extern "C" void CB_NEAR cb_bloodprg_0064a0_vm_op_d0_cond_branch(CbMachine* m)
{
    cb_u8 test_result = (cb_u8)(m->read8(m->gs, 0x252a) & 1);
    m->set_logic8_flags(test_result);
    if (test_result == 0) {
        m->call_near(0x6462);
    }
    return;
}
