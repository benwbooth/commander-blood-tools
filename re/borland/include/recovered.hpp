#ifndef CB_RECOVERED_HPP
#define CB_RECOVERED_HPP

#if defined(__BORLANDC__)
#define CB_NEAR near
#define CB_FAR far
#else
#define CB_NEAR
#define CB_FAR
#endif

typedef unsigned char cb_u8;
typedef signed char cb_i8;
typedef unsigned short cb_u16;
typedef signed short cb_i16;
typedef unsigned long cb_u32;
typedef signed long cb_i32;

struct CbMachine {
    cb_u16 ax;
    cb_u16 bx;
    cb_u16 cx;
    cb_u16 dx;
    cb_u16 si;
    cb_u16 di;
    cb_u16 bp;
    cb_u16 sp;
    cb_u16 ds;
    cb_u16 es;
    cb_u16 fs;
    cb_u16 gs;
    cb_u16 ss;
    cb_u16 cs;
    int cf;
    int zf;
    int sf;
    int of;
    int pf;
    int af;
    int df;

    cb_u8 read8(cb_u16 seg, cb_u16 off) const;
    cb_u16 read16(cb_u16 seg, cb_u16 off) const;
    void write8(cb_u16 seg, cb_u16 off, cb_u8 value);
    void write16(cb_u16 seg, cb_u16 off, cb_u16 value);
    void set_logic8_flags(cb_u8 value);
    void set_logic16_flags(cb_u16 value);
    void set_add16_flags(cb_u16 left, cb_u16 right, cb_u16 result);
    void set_sub16_flags(cb_u16 left, cb_u16 right, cb_u16 result);
    void set_dec16_flags(cb_u16 before, cb_u16 result);
    void set_sar16_flags(cb_u16 before, unsigned count, cb_u16 result);
    void jump_near(cb_u16 off);
};

inline cb_u8 cb_lo8(cb_u16 value)
{
    return (cb_u8)(value & 0xffu);
}

inline cb_u8 cb_hi8(cb_u16 value)
{
    return (cb_u8)((value >> 8) & 0xffu);
}

inline void cb_set_lo8(cb_u16& reg, cb_u8 value)
{
    reg = (cb_u16)((reg & 0xff00u) | value);
}

inline void cb_set_hi8(cb_u16& reg, cb_u8 value)
{
    reg = (cb_u16)((reg & 0x00ffu) | ((cb_u16)value << 8));
}

inline void cb_advance_u16(cb_u16& reg, cb_u16 amount, int direction_flag)
{
    if (direction_flag) {
        reg = (cb_u16)(reg - amount);
    } else {
        reg = (cb_u16)(reg + amount);
    }
}

#endif
