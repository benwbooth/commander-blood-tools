// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x007542
// assembly: re/assembly/bloodprg/seg_04da/func_007542_routine.asm
// provenance: static_dispatch_table_target
// status: translated_gs_byte_store_imm8
// reason: mechanical translation of byte parser dispatch 0x01 flag store to GS:0x0b16

#include "recovered.hpp"

extern "C" void CB_NEAR cb_bloodprg_007542_routine(CbMachine* m)
{
    m->write8(m->gs, 0xb16, 1);
    return;
}
