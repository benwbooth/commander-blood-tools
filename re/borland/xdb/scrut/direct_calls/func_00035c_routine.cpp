// Commander Blood Borland C++ translation unit
// module: xdb_scrut
// overlay_offset: 0x00035c
// assembly: re/assembly/xdb/scrut/direct_calls/func_00035c_routine.asm
// provenance: direct_call_from_0xa3
// status: translated_xdb_mouse_position
// reason: mechanical translation of XDB int 33h mouse position helper

#include "recovered.hpp"

extern "C" void CB_NEAR cb_xdb_scrut_00035c_routine(CbMachine* m)
{
    m->write16(m->ds, 0x002a, m->cx);
    m->write16(m->ds, 0x002c, m->dx);
    m->ax = 4;
    m->interrupt(0x33);
    return;
}
