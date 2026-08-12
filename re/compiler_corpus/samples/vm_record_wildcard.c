/*
 * Codegen probe for BLOODPRG 0x006946.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile u8 FAR *record_base_global;
extern volatile u8 query_mode;
extern volatile u16 wildcard_ref_value;
extern volatile u16 branch_a;

#if defined(__WATCOMC__)
#pragma aux branch_fail_probe value [si] modify exact [ax si]
#pragma aux lookup_owner_probe parm [ax] value [ax] modify exact [ax]
#pragma aux remove_owner_probe parm [ax] value [ax] modify exact [ax]
#pragma aux insert_owner_probe parm [ax] value [ax] modify exact [ax]
#pragma aux vm_record_wildcard_probe parm [si] value [si] modify exact [ax bx cx dx si es]
#endif

extern u16 NEAR branch_fail_probe(void);
extern u16 NEAR lookup_owner_probe(u16 offset);
extern int NEAR remove_owner_probe(u16 owner);
extern int NEAR insert_owner_probe(u16 owner);

const u8 NEAR *NEAR vm_record_wildcard_probe(const u8 NEAR *script_bytes)
{
    int inverted;
    u8 opcode;
    u16 offset;
    u16 value;
    u16 owner;
    volatile u8 FAR *record_base;
    volatile u16 FAR *field;

    record_base = record_base_global;
    if ((query_mode & 1u) != 0) {
        inverted = 0;
        if (*script_bytes == 0xa1u) {
            inverted = 1;
            ++script_bytes;
        }
        offset = *(const u16 NEAR *)script_bytes;
        script_bytes += sizeof(u16);
        value = *(const u16 NEAR *)script_bytes;
        script_bytes += sizeof(u16);
        if (value == wildcard_ref_value) {
            value = 0xffffu;
        }
        field = (volatile u16 FAR *)(record_base + offset);
        if ((*field == value) == inverted) {
            return (const u8 NEAR *)branch_fail_probe();
        }
        return script_bytes;
    }

    offset = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);
    value = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);
    opcode = script_bytes[-5];
    if (opcode == 0xbcu) {
        branch_a = value;
    }
    field = (volatile u16 FAR *)(record_base + offset);
    if (*field == 0xffffu) {
        owner = lookup_owner_probe(offset);
        remove_owner_probe(owner);
    } else if (value == wildcard_ref_value || value == 0xffffu) {
        owner = lookup_owner_probe(offset);
        if (!insert_owner_probe(owner)) {
            return script_bytes;
        }
        value = 0xffffu;
    }
    *field = value;
    return script_bytes;
}
