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
    u16 estimate;
    u16 next;

    if (value == 0) {
        return 0;
    }

    if ((value >> 16) != 0) {
        estimate = ((value >> 24) != 0) ? 0xffffu : 0x0fffu;
    } else {
        estimate = ((value >> 8) != 0) ? 0x00ffu : 0x000fu;
    }

    for (;;) {
        next = (u16)(((u16)(value / estimate) + estimate) >> 1);
        if (next >= estimate) {
            return estimate;
        }
        estimate = next;
    }
}
