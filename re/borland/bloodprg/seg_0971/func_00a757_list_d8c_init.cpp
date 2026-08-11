// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00a757
// assembly: re/assembly/bloodprg/seg_0971/func_00a757_list_d8c_init.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_list_d8c_init
// reason: mechanical translation of list D8C initialization stores

#include "recovered.hpp"

// label: list_d8c_init

extern "C" void CB_FAR cb_bloodprg_00a757_list_d8c_init(CbMachine* m)
{
    m->ax = m->read16(m->ds, 0x0a7e);
    m->write16(m->ds, 0x0d8e, m->ax);
    m->write16(m->ds, 0x0d92, m->ax);
    m->ax = 0;
    m->set_logic16_flags(m->ax);
    m->write16(m->ds, 0x0d8c, m->ax);
    m->write16(m->ds, 0x0d90, m->ax);
    m->write16(m->ds, 0x0d9a, m->ax);
    m->write16(m->ds, 0x0da0, m->ax);
    m->write16(m->ds, 0x0d96, m->ax);
    m->ax = m->read16(m->ds, 0x5233);
    m->write16(m->ds, 0x0d98, m->ax);
    return;
}
