#ifndef XDB_KEYBOARD_H
#define XDB_KEYBOARD_H

#include "xdb_common.h"

#if defined(__WATCOMC__)
#include <bios.h>
#define xdb_keyboard_ready() _bios_keybrd(_KEYBRD_READY)
#define xdb_keyboard_read() _bios_keybrd(_KEYBRD_READ)
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#include <bios.h>
#define xdb_keyboard_ready() bioskey(1)
#define xdb_keyboard_read() bioskey(0)
#else
xdb_u16 xdb_keyboard_ready(void);
xdb_u16 xdb_keyboard_read(void);
#endif

#endif
