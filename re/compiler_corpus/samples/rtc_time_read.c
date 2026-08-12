/* Codegen probe for BLOODPRG 0x00093B. */

#include <dos.h>

typedef signed char i8;
typedef signed int i16;
typedef unsigned char u8;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile i16 rtc_hour;
extern u8 NEAR bcd_to_binary_probe(u8 value);

#if defined(__WATCOMC__)
#pragma aux bcd_to_binary_probe parm [ax] value [al] modify [ax]
#endif

void FAR rtc_time_read_probe(void)
{
    union REGS registers;
    u8 hour;

    registers.x.ax = 0x0200;
    int86(0x1A, &registers, &registers);
    hour = bcd_to_binary_probe(registers.h.ch);
    rtc_hour = (i16)(i8)hour;
}
