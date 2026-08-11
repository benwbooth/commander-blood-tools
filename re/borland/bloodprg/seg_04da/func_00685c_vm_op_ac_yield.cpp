// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00685c
// assembly: re/assembly/bloodprg/seg_04da/func_00685c_vm_op_ac_yield.asm
// provenance: static_dispatch_table_target
// status: translated_gs_byte_store_imm8
// reason: mechanical translation of VM opcode 0xac yield flag store to GS:0x67b4

#include "recovered.hpp"

// label: vm_op_ac_yield

extern "C" void CB_NEAR cb_bloodprg_00685c_vm_op_ac_yield(CbMachine* m)
{
    m->write8(m->gs, 0x67b4, 1);
    return;
}
