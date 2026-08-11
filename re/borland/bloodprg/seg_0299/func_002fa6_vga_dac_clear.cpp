// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x002fa6
// assembly: re/assembly/bloodprg/seg_0299/func_002fa6_vga_dac_clear.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_vga_dac_clear
// reason: mechanical translation of VGA DAC zero-fill loop

#include "recovered.hpp"

// label: vga_dac_clear

extern "C" void CB_FAR cb_bloodprg_002fa6_vga_dac_clear(CbMachine* m)
{
    m->push16(m->ax);
    m->push16(m->cx);
    m->push16(m->dx);
    m->dx = 0x03c8;
    cb_set_lo8(m->ax, 0);
    m->set_logic8_flags(cb_lo8(m->ax));
    m->out8(m->dx, cb_lo8(m->ax));
    cb_u8 dl_before = cb_lo8(m->dx);
    cb_u8 dl_after = (cb_u8)(dl_before + 1);
    cb_set_lo8(m->dx, dl_after);
    m->set_inc8_flags(dl_before, dl_after);
    m->cx = 0x0300;
    while (m->cx != 0) {
        m->out8(m->dx, cb_lo8(m->ax));
        m->cx = (cb_u16)(m->cx - 1);
    }
    m->dx = m->pop16();
    m->cx = m->pop16();
    m->ax = m->pop16();
    return;
}
