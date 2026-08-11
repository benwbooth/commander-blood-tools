// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x002665
// assembly: re/assembly/bloodprg/seg_01ce/func_002665_strlen.asm
// provenance: relocation_proven_far_transfer_target
// status: translated_strlen_es_di
// reason: mechanical translation of ES:DI NUL scan preserving CX/DI

#include "recovered.hpp"

// label: strlen

extern "C" void CB_FAR cb_bloodprg_002665_strlen(CbMachine* m)
{
    cb_u16 saved_cx = m->cx;
    cb_u16 saved_di = m->di;
    cb_u16 scan_cx = 0xffffu;
    cb_u16 scan_di = m->di;

    while (scan_cx != 0) {
        cb_u8 value = m->read8(m->es, scan_di);
        scan_di = (cb_u16)(m->df ? scan_di - 1 : scan_di + 1);
        scan_cx = (cb_u16)(scan_cx - 1);
        if (value == 0) {
            break;
        }
    }

    cb_u16 neg_cx = (cb_u16)(0 - scan_cx);
    cb_u16 before_sub = neg_cx;
    m->ax = (cb_u16)(before_sub - 2);
    m->set_sub16_flags(before_sub, 2, m->ax);
    m->di = saved_di;
    m->cx = saved_cx;
    return;
}
