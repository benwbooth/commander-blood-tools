#include "../include/bloodprg_list.h"

#if defined(__WATCOMC__)
static void cb_dos_close(cb_u16 handle);
#pragma aux cb_dos_close = "mov ah,3eh" "int 21h" parm [bx] modify exact [ax]
static void cb_clear_cx(void);
#pragma aux cb_clear_cx = "xor cx,cx" modify exact [cx]
#pragma aux close_file_d5b modify exact [ax bx cx]
#pragma aux list_d8c_bounds_init modify exact []
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#include <io.h>
#define cb_dos_close(handle) ((void)close((int)(handle)))
#else
extern int close(int handle);
#define cb_dos_close(handle) ((void)close((int)(handle)))
#endif

void CB_NEAR close_file_d5b(void)
{
    cb_u16 handle = list_d8c_file_handle;

    if (handle != 0 && handle != list_d8c_reserved_file_handle) {
        list_d8c_file_handle = 0;
        cb_dos_close(handle);
        list_d8c_bounds_init();
    }

#if defined(__WATCOMC__)
    cb_clear_cx();
#elif defined(__TURBOC__) || defined(__BORLANDC__)
    asm xor cx, cx;
#endif
}
