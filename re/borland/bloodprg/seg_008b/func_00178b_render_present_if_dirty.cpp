// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00178b
// assembly: re/assembly/bloodprg/seg_008b/func_00178b_render_present_if_dirty.asm
// provenance: recursive_graph
// status: translated_render_present_if_dirty
// reason: mechanical translation of dirty-flag gated display far-call sequence

#include "recovered.hpp"

// label: render_present_if_dirty

extern "C" void CB_NEAR cb_bloodprg_00178b_render_present_if_dirty(CbMachine* m)
{
    cb_u8 test_result = (cb_u8)(m->read8(m->ds, 0x5b55) & 1);
    m->set_logic8_flags(test_result);
    if (test_result != 0) {
        m->call_far(0x0000, 0x05d7);
        m->si = 0x5251;
        m->call_far(0x0299, 0x0000);
        m->write8(m->ds, 0x5b55, 0);
        m->write8(m->ds, 0x0a40, 0);
        m->write8(m->ds, 0x0a3e, 0);
    }
    return;
}
