// Commander Blood Borland C++ translation unit
// module: bloodprg
// file_offset: 0x00267d
// assembly: re/assembly/bloodprg/seg_01ce/func_00267d_kbd_read_int16.asm
// provenance: recursive_graph, relocation_proven_far_transfer_target
// status: translated_kbd_read_int16
// reason: mechanical translation of BIOS int 16h keyboard poll/read

#include "recovered.hpp"

// label: kbd_read_int16

extern "C" void CB_FAR cb_bloodprg_00267d_kbd_read_int16(CbMachine* m)
{
    m->ax = 0x0100;
    m->interrupt(0x16);
    if (!m->zf) {
        m->ax = 0;
        m->set_logic16_flags(m->ax);
        m->interrupt(0x16);
        return;
    }
    m->ax = 0;
    m->set_logic16_flags(m->ax);
    return;
}
