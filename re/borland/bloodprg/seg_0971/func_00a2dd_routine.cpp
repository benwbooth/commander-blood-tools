// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a2dd
// assembly: re/assembly/bloodprg/seg_0971/func_00a2dd_routine.asm
// provenance: recursive_graph
// status: translated_resource_empty_close_gate
// reason: mechanical translation of queue-empty flag update plus close call

#include "recovered.hpp"

extern "C" void CB_NEAR cb_bloodprg_00a2dd_routine(CbMachine* m)
{
    cb_u8 flags = m->read8(m->ds, 0x0d5f);
    cb_u8 flags_result = (cb_u8)(flags | 1);
    m->write8(m->ds, 0x0d5f, flags_result);
    m->set_logic8_flags(flags_result);
    cb_u16 count = m->read16(m->ds, 0x0d9a);
    m->set_sub16_flags(count, 0, count);
    if (count == 0) {
        flags = m->read8(m->ds, 0x0d5f);
        flags_result = (cb_u8)(flags | 2);
        m->write8(m->ds, 0x0d5f, flags_result);
        m->set_logic8_flags(flags_result);
        m->call_near(0xa141);
    }
    return;
}
