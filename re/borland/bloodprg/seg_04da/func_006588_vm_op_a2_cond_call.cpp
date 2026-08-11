// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x006588
// assembly: re/assembly/bloodprg/seg_04da/func_006588_vm_op_a2_cond_call.asm
// provenance: static_dispatch_table_target
// status: translated_vm_op_a2_random_branch
// reason: mechanical translation of LODSW, PRNG far call, OR AX,AX, conditional VM branch call

#include "recovered.hpp"

// label: vm_op_a2_cond_call

extern "C" void CB_NEAR cb_bloodprg_006588_vm_op_a2_cond_call(CbMachine* m)
{
    m->ax = m->read16(m->ds, m->si);
    cb_advance_u16(m->si, 2, m->df);
    m->call_far(0x01ce, 0x0b02);
    m->set_logic16_flags(m->ax);
    if (m->ax != 0) {
        m->call_near(0x6462);
    }
    return;
}
