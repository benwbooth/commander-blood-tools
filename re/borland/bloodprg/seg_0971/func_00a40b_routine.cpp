// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a40b
// assembly: re/assembly/bloodprg/seg_0971/func_00a40b_routine.asm
// provenance: recursive_graph
// status: translated_gs_d5f_compare_zero_or_one
// reason: mechanical translation of two-stage GS:0x0d5f byte compare

#include "recovered.hpp"

extern "C" void CB_NEAR cb_bloodprg_00a40b_routine(CbMachine* m)
{
    cb_u8 first_value = m->read8(m->gs, 0x0d5f);
    cb_u8 first_result = first_value;
    m->set_sub8_flags(first_value, 0, first_result);
    if (first_result == 0) {
        return;
    }
    cb_u8 second_value = m->read8(m->gs, 0x0d5f);
    cb_u8 second_result = (cb_u8)(second_value - 1);
    m->set_sub8_flags(second_value, 1, second_result);
    return;
}
