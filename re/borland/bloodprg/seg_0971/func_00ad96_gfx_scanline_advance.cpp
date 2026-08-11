// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00ad96
// assembly: re/assembly/bloodprg/seg_0971/func_00ad96_gfx_scanline_advance.asm
// provenance: recursive_graph
// status: translated_gfx_scanline_advance
// reason: mechanical translation of row-counter/scanline advance with zero-row epilogue

#include "recovered.hpp"

// label: gfx_scanline_advance

extern "C" void CB_NEAR cb_bloodprg_00ad96_gfx_scanline_advance(CbMachine* m)
{
    cb_u16 row_off = (cb_u16)(m->bp - 6);
    cb_u8 row_count = (cb_u8)(m->read8(m->ss, row_off) - 1);
    m->write8(m->ss, row_off, row_count);
    if (row_count == 0) {
        cb_u16 before_add = m->sp;
        m->sp = (cb_u16)(m->sp + 2);
        m->set_add16_flags(before_add, 2, m->sp);
        m->sp = m->bp;
        m->bp = m->pop16();
        m->ds = m->pop16();
        return;
    }
    m->di = m->read16(m->ss, (cb_u16)(m->bp - 8));
    cb_u16 before_add = m->di;
    m->di = (cb_u16)(m->di + 0x0140);
    m->set_add16_flags(before_add, 0x0140, m->di);
    m->cx = m->read16(m->ss, (cb_u16)(m->bp - 0x0a));
    m->write16(m->ss, (cb_u16)(m->bp - 8), m->di);
    return;
}
