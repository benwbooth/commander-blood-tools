/* Codegen probe for BLOODPRG 0x000D61. */

#include <dos.h>

typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

#if defined(__WATCOMC__)
#pragma aux print_string_dos_probe parm [si]
#endif

void FAR print_string_dos_probe(const volatile char *text)
{
    u16 index;

    index = 0;
    while (text[index] != '\0') {
        bdos(2, (u8)text[index], 0);
        ++index;
    }
}
