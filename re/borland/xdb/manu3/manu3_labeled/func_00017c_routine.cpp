// Commander Blood Borland C++ translation unit
// module: xdb_manu3
// overlay_offset: 0x00017c
// assembly: re/assembly/xdb/manu3/manu3_labeled/func_00017c_routine.asm
// provenance: manu3 selector wrapper entry
// status: translated_manu3_selector_wrapper
// reason: mechanical translation of near call to MANU3 selector followed by far return

#include "recovered.hpp"

extern "C" void CB_FAR cb_xdb_manu3_00017c_routine(CbMachine* m)
{
    m->call_near(0x0181);
    return;
}
