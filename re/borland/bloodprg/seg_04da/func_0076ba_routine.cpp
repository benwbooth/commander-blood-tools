// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x0076ba
// assembly: re/assembly/bloodprg/seg_04da/func_0076ba_routine.asm
// provenance: static_dispatch_table_target
// status: translated_lodsw_store_gs_1fa5
// reason: mechanical translation of lodsw plus GS:0x1fa5 store

#include "recovered.hpp"

extern "C" void CB_NEAR cb_bloodprg_0076ba_routine(CbMachine* m)
{
    m->ax = m->read16(m->ds, m->si);
    cb_advance_u16(m->si, 2, m->df);
    m->write16(m->gs, 0x1fa5, m->ax);
    return;
}
