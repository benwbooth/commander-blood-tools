// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x007788
// assembly: re/assembly/bloodprg/seg_04da/func_007788_fs_name_area_read.asm
// provenance: static_dispatch_table_target
// status: translated_fs_name_area_read
// reason: mechanical translation of ES=FS control-byte copy preserving ES

#include "recovered.hpp"

// label: fs_name_area_read

extern "C" void CB_NEAR cb_bloodprg_007788_fs_name_area_read(CbMachine* m)
{
    cb_u16 saved_es = m->es;
    m->ax = m->fs;
    m->es = m->ax;
    m->di = 0x0c74;
    for (;;) {
        cb_set_lo8(m->ax, m->read8(m->ds, m->si));
        cb_advance_u16(m->si, 1, m->df);
        cb_u8 value = cb_lo8(m->ax);
        m->set_logic8_flags(value);
        if ((value & 0x80u) != 0) {
            break;
        }
        cb_u8 cmp_result = (cb_u8)(value - 0x20);
        m->set_sub8_flags(value, 0x20, cmp_result);
        if (value < 0x20u) {
            break;
        }
        m->write8(m->es, m->di, value);
        cb_advance_u16(m->di, 1, m->df);
    }
    cb_u16 before_dec = m->si;
    m->si = (cb_u16)(m->si - 1);
    m->set_dec16_flags(before_dec, m->si);
    m->write8(m->es, m->di, 0);
    m->write8(m->gs, 0x27e8, 1);
    m->es = saved_es;
    return;
}
