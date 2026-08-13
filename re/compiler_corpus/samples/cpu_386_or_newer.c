/* Codegen probe for BLOODPRG 0x000CCB. */

typedef unsigned int u16;

extern u16 near flags_read(void);
extern void near flags_write(u16 flags);

#if defined(__WATCOMC__)
#pragma aux flags_read = "pushf" "pop ax" value [ax] modify exact [ax]
#pragma aux flags_write = "push ax" "popf" parm [ax] modify exact []
#endif

u16 far cpu_386_or_newer_probe(void)
{
    u16 original_flags;
    u16 observed_flags;
    u16 supported;

    original_flags = flags_read();
    supported = 0u;
    flags_write(0u);
    observed_flags = flags_read();
    if ((observed_flags & 0xf000u) != 0xf000u) {
        flags_write(0x7000u);
        observed_flags = flags_read();
        if ((observed_flags & 0x7000u) != 0u) {
            supported = 1u;
        }
    }
    flags_write(original_flags);
    return supported;
}
