/*
 * Codegen probe for BLOODPRG 0x002665.
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

u16 FAR_FN far_strlen_probe(const char FAR *s)
{
    const char FAR *p;

    p = s;
    while (*p != '\0') {
        ++p;
    }

    return (u16)(p - s);
}
