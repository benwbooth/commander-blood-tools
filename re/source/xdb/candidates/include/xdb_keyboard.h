#ifndef XDB_KEYBOARD_H
#define XDB_KEYBOARD_H

#include "xdb_common.h"

#if defined(__WATCOMC__)
extern xdb_u16 XDB_NEAR xdb_keyboard_ready(void);
#pragma aux xdb_keyboard_ready = \
        "mov ah,1" \
        "int 16h" \
        "setnz al" \
        "xor ah,ah" \
        value [ax] \
        modify exact [ax]
extern xdb_u16 XDB_NEAR xdb_keyboard_read(void);
#pragma aux xdb_keyboard_read = \
        "xor ah,ah" \
        "int 16h" \
        value [ax] \
        modify exact [ax]
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#include <bios.h>
#define xdb_keyboard_ready() bioskey(1)
#define xdb_keyboard_read() bioskey(0)
#else
xdb_u16 xdb_keyboard_ready(void);
xdb_u16 xdb_keyboard_read(void);
#endif

#endif
