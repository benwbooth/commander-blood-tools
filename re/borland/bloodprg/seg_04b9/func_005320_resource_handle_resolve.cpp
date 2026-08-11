// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x005320
// assembly: re/assembly/bloodprg/seg_04b9/func_005320_resource_handle_resolve.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_resource_handle_resolve
// reason: mechanical translation of FS resource-handle table resolver

#include "recovered.hpp"

// label: resource_handle_resolve

extern "C" void CB_FAR cb_bloodprg_005320_resource_handle_resolve(CbMachine* m)
{
    cb_u16 saved_bx = m->bx;
    cb_u16 table_off = (cb_u16)(m->ax << 3);
    m->bx = table_off;
    m->ax = 0;
    m->set_logic16_flags(m->ax);
    cb_u16 flags = m->read16(m->fs, (cb_u16)(m->bx + 2));
    cb_u16 test_result = (cb_u16)(flags & 3);
    m->set_logic16_flags(test_result);
    if (test_result != 0) {
        m->ax = m->read16(m->fs, m->bx);
        m->ds = m->ax;
        m->si = 0;
        m->set_logic16_flags(m->si);
        m->ax = 1;
    }
    m->bx = saved_bx;
    return;
}
