/*
 * Codegen probe for BLOODPRG 0x006510.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;
typedef signed char i8;

typedef union tagged_byte_pair {
    u16 word;
    struct {
        i8 low;
        i8 high;
    } bytes;
} tagged_byte_pair;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

extern volatile i8 compare_pair_low;
extern volatile i8 compare_pair_high;
u16 NEAR vm_branch_probe(void);

#if defined(__WATCOMC__)
#pragma aux vm_branch_probe value [si] modify exact [ax si]
#pragma aux vm_tagged_byte_pair_compare_probe parm [si] value [si] modify exact [ax bx dx si]
#endif

const u8 NEAR *NEAR vm_tagged_byte_pair_compare_probe(
        const u8 NEAR *script_bytes)
{
    u8 tag;
    tagged_byte_pair pair;

    tag = *script_bytes++;
    pair.word = *(const u16 NEAR *)script_bytes;
    script_bytes += 4;

    if (tag == 0xf1u) {
        if (pair.bytes.high < compare_pair_high) {
            goto failed;
        }
        if (pair.bytes.high > compare_pair_high) {
            return script_bytes;
        }
        if (pair.bytes.low <= compare_pair_low) {
            goto failed;
        }
        return script_bytes;
    } else if (tag == 0xf2u) {
        if (pair.bytes.high > compare_pair_high) {
            goto failed;
        }
        if (pair.bytes.high < compare_pair_high) {
            return script_bytes;
        }
        if (pair.bytes.low >= compare_pair_low) {
            goto failed;
        }
        return script_bytes;
    } else {
        if (pair.bytes.high != compare_pair_high) {
            goto failed;
        }
        if (pair.bytes.low == compare_pair_low) {
            return script_bytes;
        }
    }

failed:
    return (const u8 NEAR *)vm_branch_probe();
}
