// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x005288
// assembly: re/assembly/bloodprg/seg_04b9/func_005288_resource_release.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_resource_release
// reason: mechanical translation of resource loaded-flag test plus CS-pushed free call

#include "recovered.hpp"

// label: resource_release

extern "C" void CB_FAR cb_bloodprg_005288_resource_release(CbMachine* m)
{
    m->push16(m->bx);
    m->bx = m->ax;
    m->bx = (cb_u16)(m->bx << 3);
    cb_u16 test_result = (cb_u16)(m->read16(m->fs, (cb_u16)(m->bx + 2)) & 3);
    m->set_logic16_flags(test_result);
    if (test_result != 0) {
        m->push16(m->cs);
        m->call_near(0x529c);
    }
    m->bx = m->pop16();
    return;
}
