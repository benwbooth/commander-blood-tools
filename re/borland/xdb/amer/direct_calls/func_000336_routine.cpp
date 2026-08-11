// Commander Blood Borland C++ translation unit
// module: xdb_amer
// overlay_offset: 0x000336
// assembly: re/assembly/xdb/amer/direct_calls/func_000336_routine.asm
// provenance: direct_call_from_0xa3
// status: translated_xdb_mouse_range
// reason: mechanical translation of XDB int 33h mouse range helper

#include "recovered.hpp"

extern "C" void CB_NEAR cb_xdb_amer_000336_routine(CbMachine* m)
{
    m->push16(m->cx);
    m->ax = 8;
    m->cx = 0;
    m->set_logic16_flags(m->cx);
    m->interrupt(0x33);
    m->ax = 7;
    m->dx = m->pop16();
    m->cx = 0;
    m->set_logic16_flags(m->cx);
    m->interrupt(0x33);
    return;
}
