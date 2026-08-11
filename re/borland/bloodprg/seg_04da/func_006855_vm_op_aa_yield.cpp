// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x006855
// assembly: re/assembly/bloodprg/seg_04da/func_006855_vm_op_aa_yield.asm
// provenance: static_dispatch_table_target
// status: translated_gs_byte_store_imm8
// reason: mechanical translation of VM opcode 0xaa yield flag store to GS:0x67b4

#include "recovered.hpp"

// label: vm_op_aa_yield

extern "C" void CB_NEAR cb_bloodprg_006855_vm_op_aa_yield(CbMachine* m)
{
    m->write8(m->gs, 0x67b4, 1);
    return;
}
