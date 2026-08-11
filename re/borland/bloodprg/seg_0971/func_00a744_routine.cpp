// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a744
// assembly: re/assembly/bloodprg/seg_0971/func_00a744_routine.asm
// provenance: recursive_graph
// status: translated_list_d8c_bounds_tail
// reason: mechanical translation of list bound tail stores at DS:0x0d62..0x0d66

#include "recovered.hpp"

extern "C" void CB_NEAR cb_bloodprg_00a744_routine(CbMachine* m)
{
    m->write16(m->ds, 0x0d62, 0);
    m->write16(m->ds, 0x0d64, 0xffff);
    m->write16(m->ds, 0x0d66, 0xffff);
    return;
}
