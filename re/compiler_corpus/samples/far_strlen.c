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

u16 FAR_FN far_strlen_probe(const char FAR *s);

#if defined(__WATCOMC__)
#pragma aux far_strlen_probe parm [es di] value [ax] modify exact [ax]
#endif

u16 FAR_FN far_strlen_probe(const char FAR *s)
{
    u16 length;

    length = 0;
    while (length != 0xffffu) {
        if (*s == '\0') {
            return length;
        }
        ++s;
        ++length;
    }

    return 0xfffeu;
}
