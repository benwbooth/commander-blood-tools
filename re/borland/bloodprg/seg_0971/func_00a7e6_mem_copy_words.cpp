// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a7e6
// assembly: re/assembly/bloodprg/seg_0971/func_00a7e6_mem_copy_words.asm
// provenance: recursive_graph
// status: translated_mem_copy_words_4
// reason: mechanical translation of push ds/pop es plus four MOVSW instructions

#include "recovered.hpp"

// label: mem_copy_words

extern "C" void CB_NEAR cb_bloodprg_00a7e6_mem_copy_words(CbMachine* m)
{
    m->es = m->ds;
    for (int i = 0; i != 4; ++i) {
        cb_u16 value = m->read16(m->ds, m->si);
        m->write16(m->es, m->di, value);
        if (m->df) {
            m->si = (cb_u16)(m->si - 2);
            m->di = (cb_u16)(m->di - 2);
        } else {
            m->si = (cb_u16)(m->si + 2);
            m->di = (cb_u16)(m->di + 2);
        }
    }
    return;
}
