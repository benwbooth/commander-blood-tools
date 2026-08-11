// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x009510
// assembly: re/assembly/bloodprg/seg_071e/func_009510_presentation_mode_check.asm
// provenance: recursive_graph
// status: translated_presentation_mode_check
// reason: mechanical translation of presentation mode bucket update from DS:0x2795

#include "recovered.hpp"

// label: presentation_mode_check

extern "C" void CB_NEAR cb_bloodprg_009510_presentation_mode_check(CbMachine* m)
{
    m->push16(m->bx);
    m->push16(m->dx);
    m->ax = m->read16(m->ds, 0x2793);
    m->ax = (cb_u16)(m->ax & 0xff0f);
    m->set_logic16_flags(m->ax);
    cb_u16 active = (cb_u16)(m->ax & 2);
    m->set_logic16_flags(active);
    if (active == 0) {
        m->bx = 1;
        m->dx = m->read16(m->ds, 0x2795);
        cb_u16 cmp_result = (cb_u16)(m->dx - 0x0016);
        m->set_sub16_flags(m->dx, 0x0016, cmp_result);
        if ((cb_i16)m->dx > (cb_i16)0x0016) {
            cmp_result = (cb_u16)(m->dx - 0x009d);
            m->set_sub16_flags(m->dx, 0x009d, cmp_result);
            if ((cb_i16)m->dx <= (cb_i16)0x009d) {
                cb_u16 before_add = m->bx;
                m->bx = (cb_u16)(m->bx + m->bx);
                m->set_add16_flags(before_add, before_add, m->bx);
                cmp_result = (cb_u16)(m->dx - 0x0043);
                m->set_sub16_flags(m->dx, 0x0043, cmp_result);
                if ((cb_i16)m->dx > (cb_i16)0x0043) {
                    before_add = m->bx;
                    m->bx = (cb_u16)(m->bx + m->bx);
                    m->set_add16_flags(before_add, before_add, m->bx);
                    cmp_result = (cb_u16)(m->dx - 0x0070);
                    m->set_sub16_flags(m->dx, 0x0070, cmp_result);
                    if ((cb_i16)m->dx > (cb_i16)0x0070) {
                        before_add = m->bx;
                        m->bx = (cb_u16)(m->bx + m->bx);
                        m->set_add16_flags(before_add, before_add, m->bx);
                    }
                }
            }
        }
        m->bx = (cb_u16)(m->bx << 4);
        m->ax = (cb_u16)(m->ax | m->bx);
        m->set_logic16_flags(m->ax);
    }
    m->write16(m->ds, 0x2793, m->ax);
    m->dx = m->pop16();
    m->bx = m->pop16();
    return;
}
