/*
 * Codegen probe for BLOODPRG 0x002E33.
 * This is not recovered game source.
 */
typedef unsigned int u16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR_FN far
#else
#define FAR_FN
#endif

u16 FAR_FN u32_sqrt_newton_probe(u32 value)
{
    u16 low;
    u16 high;
    u16 estimate;

    low = (u16)value;
    high = (u16)(value >> 16);

    if (high != 0) {
        estimate = 0x0fffu;
        if ((high & 0xff00u) != 0) {
            estimate = 0xffffu;
            if (high >= 0xfffeu) {
                return low;
            }
        }
    } else {
        if (low == 0) {
            return low;
        }
        estimate = 0x000fu;
        if ((low & 0xff00u) != 0) {
            estimate = 0x00ffu;
        }
    }

    for (;;) {
        u16 quotient;
        u16 candidate;

        quotient = (u16)(value / estimate);
        candidate = (u16)(((u32)quotient + estimate) >> 1);
        if (candidate >= estimate) {
            return candidate;
        }
        estimate = candidate;
    }
}
