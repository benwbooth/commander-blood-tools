/*
 * Codegen probe for BLOODPRG 0x0025A4.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define FAR_FN far
#else
#define FAR
#define FAR_FN
#endif

int FAR_FN string_equal_mixed_probe(const char *left, const char FAR *right)
{
    while (*left == *right) {
        if (*left == '\0') {
            return 1;
        }
        ++left;
        ++right;
    }

    return 0;
}
