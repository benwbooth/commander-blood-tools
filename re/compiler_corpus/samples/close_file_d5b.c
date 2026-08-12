/*
 * Codegen probe for BLOODPRG 0x00A141.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile u16 file_handle;
extern volatile u16 reserved_file_handle;
void NEAR bounds_init_probe(void);

#if defined(__WATCOMC__)
static void dos_close_handle(u16 handle);
#pragma aux dos_close_handle = "mov ah,3eh" "int 21h" parm [bx] modify exact [ax]
static void clear_cx(void);
#pragma aux clear_cx = "xor cx,cx" modify exact [cx]
#pragma aux close_file_d5b_probe modify exact [ax bx cx]
#pragma aux bounds_init_probe modify exact []
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#include <io.h>
#define dos_close_handle(handle) ((void)close((int)(handle)))
#endif

void NEAR close_file_d5b_probe(void)
{
    u16 handle = file_handle;

    if (handle != 0 && handle != reserved_file_handle) {
        file_handle = 0;
        dos_close_handle(handle);
        bounds_init_probe();
    }

#if defined(__WATCOMC__)
    clear_cx();
#elif defined(__TURBOC__) || defined(__BORLANDC__)
    asm xor cx, cx;
#endif
}
