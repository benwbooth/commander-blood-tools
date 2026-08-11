// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x007549
// assembly: re/assembly/bloodprg/seg_04da/func_007549_routine.asm
// provenance: static_dispatch_table_target
// status: translated_gs_byte_store_imm8
// reason: mechanical translation of byte parser dispatch 0x02 flag store to GS:0x0b16

#include "recovered.hpp"

extern "C" void CB_NEAR cb_bloodprg_007549_routine(CbMachine* m)
{
    m->write8(m->gs, 0xb16, 1);
    return;
}
