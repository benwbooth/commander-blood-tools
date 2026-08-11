// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a73e
// assembly: re/assembly/bloodprg/seg_0971/func_00a73e_list_d8c_bounds_init.asm
// provenance: recursive_graph
// status: translated_list_d8c_bounds_init
// reason: mechanical translation of list bound initialization plus fall-through tail stores

#include "recovered.hpp"

// label: list_d8c_bounds_init

extern "C" void CB_NEAR cb_bloodprg_00a73e_list_d8c_bounds_init(CbMachine* m)
{
    m->write16(m->ds, 0x0d60, 0);
    m->write16(m->ds, 0x0d62, 0);
    m->write16(m->ds, 0x0d64, 0xffff);
    m->write16(m->ds, 0x0d66, 0xffff);
    return;
}
