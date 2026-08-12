/*
 * Codegen probe for BLOODPRG 0x0067A7.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#if defined(__WATCOMC__)
#pragma aux strlen_b_probe parm [es di] value [ax] modify exact [ax]
#endif

u16 NEAR strlen_b_probe(const char FAR *text)
{
    u16 length;

    length = 0;
    while (length != 0xffffu) {
        if (*text == '\0') {
            return length;
        }
        ++text;
        ++length;
    }

    return 0xfffeu;
}
