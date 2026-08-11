// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a778
// assembly: re/assembly/bloodprg/seg_0971/func_00a778_routine.asm
// provenance: recursive_graph
// status: translated_list_d8c_call_a0c3
// reason: mechanical translation of LES setup plus near call to 0xa0c3

#include "recovered.hpp"

extern "C" void CB_NEAR cb_bloodprg_00a778_routine(CbMachine* m)
{
    m->si = m->read16(m->ds, 0x0d8c);
    m->es = m->read16(m->ds, 0x0d8e);
    m->si = m->read16(m->ds, 0x0d9e);
    m->call_near(0xa0c3);
    return;
}
