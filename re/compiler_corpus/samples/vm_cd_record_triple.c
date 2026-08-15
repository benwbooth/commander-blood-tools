/*
 * Codegen probe for BLOODPRG 0x0069C7.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed int i16;

#if defined(__WATCOMC__)
#include <dos.h>
#define FAR far
#define NEAR near
#define GAME_DATA __based(__segname("GAME_DATA"))
#define RECORD_AT(base, offset) \
    ((volatile u8 FAR *)MK_FP(FP_SEG(base), (offset)))
#elif defined(__TURBOC__) || defined(__BORLANDC__)
#include <dos.h>
#define FAR far
#define NEAR near
#define GAME_DATA far
#define RECORD_AT(base, offset) \
    ((volatile u8 FAR *)MK_FP(FP_SEG(base), (offset)))
#else
#define FAR
#define NEAR
#define GAME_DATA
#define RECORD_AT(base, offset) ((base) + (offset))
#endif

extern volatile u8 FAR * GAME_DATA record_base_global;
extern volatile u8 GAME_DATA query_mode;
extern volatile u16 GAME_DATA wildcard_ref_value;
extern volatile u8 GAME_DATA ui_flags;
extern volatile u8 GAME_DATA presentation_request_flags;
extern volatile u8 GAME_DATA c2_presentation_gate;
extern volatile u16 GAME_DATA active_line;

#if defined(__WATCOMC__)
#pragma aux branch_fail_probe value [si] modify exact [ax si]
#pragma aux lookup_owner_probe parm [ax] value [ax] modify exact [ax]
#pragma aux field_offset_probe parm [ax] [bx] value [ax] modify exact [ax]
#pragma aux remove_owner_probe parm [ax] value [ax] modify exact [ax]
#pragma aux insert_owner_probe parm [ax] value [ax] modify exact [ax]
#pragma aux c2_lookup_probe parm [es di] value [ax] modify exact [ax]
#pragma aux vm_cd_record_triple_probe parm [si] value [si] modify exact [ax bx cx dx si bp]
#endif

extern u16 NEAR branch_fail_probe(void);
extern u16 NEAR lookup_owner_probe(u16 offset);
extern int NEAR field_offset_probe(u16 selector, u16 kind);
extern int NEAR remove_owner_probe(u16 owner);
extern int NEAR insert_owner_probe(u16 owner);
extern int FAR c2_lookup_probe(const volatile u8 FAR *record_name);

const u8 NEAR *NEAR vm_cd_record_triple_probe(const u8 NEAR *script_bytes)
{
    u8 inverted;
    u16 first_record;
    u16 second_record;
    u16 third_record;
    u16 owner;
    u16 kind;
    i16 field_offset;
    u16 value;
    volatile u8 FAR *record_base;
    volatile u16 FAR *triple;

    record_base = record_base_global;
    if ((query_mode & 1u) != 0) {
        inverted = 0;
        if (*script_bytes == 0xa1u) {
            inverted = 1;
            ++script_bytes;
        }
        first_record = *(const u16 NEAR *)script_bytes;
        script_bytes += sizeof(u16);
        second_record = *(const u16 NEAR *)script_bytes;
        script_bytes += sizeof(u16);
        third_record = *(const u16 NEAR *)script_bytes;
        script_bytes += sizeof(u16);
        triple = (volatile u16 FAR *)RECORD_AT(record_base, first_record);
        if (triple[0] == 0xcdu
                && triple[1] == second_record
                && triple[2] == third_record) {
            if (!inverted) {
                return script_bytes;
            }
        } else if (inverted) {
            return script_bytes;
        }
        return (const u8 NEAR *)branch_fail_probe();
    }

    first_record = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);
    owner = lookup_owner_probe(first_record);
    second_record = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);
    third_record = *(const u16 NEAR *)script_bytes;
    script_bytes += sizeof(u16);

    (void)(*(volatile u8 FAR *)RECORD_AT(record_base, owner + 2u) & 1u);
    (void)(*(volatile u8 FAR *)RECORD_AT(record_base, third_record + 2u) & 1u);
    (void)(*(volatile u8 FAR *)RECORD_AT(record_base, second_record + 2u) & 1u);

    kind = *(volatile u16 FAR *)RECORD_AT(record_base, second_record);
    (void)field_offset_probe(0x11u, kind);
    if (owner == wildcard_ref_value) {
        remove_owner_probe(second_record);
    }

    kind = *(volatile u16 FAR *)RECORD_AT(record_base, second_record);
    field_offset = (i16)field_offset_probe(0x11u, kind);
    value = third_record;
    if (third_record == wildcard_ref_value) {
        if (!insert_owner_probe(second_record)) {
            return script_bytes;
        }
        value = 0xffffu;
    }
    *(volatile u16 FAR *)RECORD_AT(
        record_base, (u16)(second_record + field_offset)) = value;

    if ((ui_flags & 1u) != 0
            || (presentation_request_flags & 2u) != 0
            || kind != 0x0400u) {
        return script_bytes;
    }
    if (c2_lookup_probe(RECORD_AT(record_base, second_record + 4u)) != 0) {
        c2_presentation_gate = 0;
        presentation_request_flags |= 2u;
        active_line = 0x2bu;
    }
    return script_bytes;
}
