/*
 * Codegen probe for BLOODPRG 0x0025A4.
 * This is not recovered game source.
 */
typedef unsigned char u8;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define FAR_FN far
#else
#define FAR
#define FAR_FN
#endif

int FAR_FN string_equal_mixed_probe(
        const volatile char FAR *left,
        const volatile char FAR *right);

int FAR_FN string_equal_mixed_probe(
        const volatile char FAR *left,
        const volatile char FAR *right)
{
    u8 ch;

    for (;;) {
        ch = (u8)*left;
        if (ch != (u8)*right) {
            return 0;
        }
        if (ch == 0) {
            return 1;
        }
        ++left;
        ++right;
    }
}
