// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x003b45
// assembly: re/assembly/bloodprg/seg_0299/func_003b45_composite_draw_a.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_composite_draw_a
// reason: mechanical translation of same-segment far-return draw wrapper

#include "recovered.hpp"

// label: composite_draw_a

extern "C" void CB_FAR cb_bloodprg_003b45_composite_draw_a(CbMachine* m)
{
    m->push16(m->cx);
    m->push16(m->cs);
    m->call_near(0x32ac);
    cb_u16 tmp = m->bp;
    m->bp = m->dx;
    m->dx = tmp;
    m->push16(m->cs);
    m->call_near(0x3321);
    cb_u16 before_add = m->bx;
    m->bx = (cb_u16)(m->bx + m->bp);
    m->set_add16_flags(before_add, m->bp, m->bx);
    cb_u16 before_dec = m->bx;
    m->bx = (cb_u16)(m->bx - 1);
    m->set_dec16_flags(before_dec, m->bx);
    m->push16(m->cs);
    m->call_near(0x3321);
    cb_u16 before_sub = m->bx;
    m->bx = (cb_u16)(m->bx - m->bp);
    m->set_sub16_flags(before_sub, m->bp, m->bx);
    cb_u16 before_inc = m->bx;
    m->bx = (cb_u16)(m->bx + 1);
    m->set_inc16_flags(before_inc, m->bx);
    tmp = m->bp;
    m->bp = m->dx;
    m->dx = tmp;
    before_add = m->cx;
    m->cx = (cb_u16)(m->cx + m->bp);
    m->set_add16_flags(before_add, m->bp, m->cx);
    before_dec = m->cx;
    m->cx = (cb_u16)(m->cx - 1);
    m->set_dec16_flags(before_dec, m->cx);
    m->push16(m->cs);
    m->call_near(0x32ac);
    m->cx = m->pop16();
    return;
}
