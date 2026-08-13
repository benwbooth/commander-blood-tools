/* Codegen probe for BLOODPRG 0x000950. */

#include <dos.h>

typedef signed char i8;
typedef unsigned char u8;
typedef signed int i16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile i16 rtc_day;
extern volatile i16 rtc_month;
extern volatile i16 rtc_year;
extern u8 NEAR bcd_to_binary(u8 value);

#if defined(__WATCOMC__)
#pragma aux bcd_to_binary parm [ax] value [al] modify [ax]
#endif

void FAR rtc_date_read_probe(void)
{
    union REGS registers;
    i16 year;

    registers.h.ah = 4u;
    int86(0x1a, &registers, &registers);

    rtc_day = (i16)(i8)bcd_to_binary(registers.h.dl);
    rtc_month = (i16)(i8)bcd_to_binary(registers.h.dh);
    year = (i16)(i8)bcd_to_binary(registers.h.cl);
    if (registers.h.ch == 0x13u) {
        year = (i16)(year + 1900);
    } else {
        year = (i16)(year + 2000);
    }
    rtc_year = year;
}
