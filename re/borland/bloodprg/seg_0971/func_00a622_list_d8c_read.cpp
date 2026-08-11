// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a622
// assembly: re/assembly/bloodprg/seg_0971/func_00a622_list_d8c_read.asm
// provenance: recursive_graph
// status: translated_list_d8c_read
// reason: mechanical translation of queue read helper and conditional LES result fetch

#include "recovered.hpp"

// label: list_d8c_read

extern "C" void CB_NEAR cb_bloodprg_00a622_list_d8c_read(CbMachine* m)
{
    m->cx = 2;
    m->call_near(0xa664);
    if (!m->cf) {
        m->si = m->read16(m->gs, 0x0d8c);
        m->es = m->read16(m->gs, 0x0d8e);
        m->ax = m->read16(m->es, (cb_u16)(m->si - 2));
    }
    return;
}
