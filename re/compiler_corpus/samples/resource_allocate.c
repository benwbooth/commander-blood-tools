/*
 * Codegen probe for BLOODPRG 0x005190.
 * This is not recovered game source.
 */
#include <dos.h>

typedef unsigned char u8;
typedef signed int i16;
typedef unsigned int u16;
typedef signed long i32;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

typedef struct resource_entry {
    u16 segment;
    u16 flags;
    u32 size;
} resource_entry;

typedef struct allocation_result {
    i16 status;
    volatile u8 FAR *destination;
} allocation_result;

extern volatile resource_entry entries[];
extern volatile u16 residents[];
extern volatile u16 evictions[];
extern volatile u16 current_handle;
extern volatile u16 current_entry_offset;
extern volatile u32 free_bytes;
extern volatile u16 pool_end_segment;

void FAR free_probe(u16 handle);
void FAR failure_probe(u16 error_code);

allocation_result FAR resource_allocate_probe(u16 handle, u32 byte_count)
{
    allocation_result result;
    volatile resource_entry *entry;
    volatile resource_entry *candidate_entry;
    volatile resource_entry FAR *special_entry;
    u32 rounded;
    u32 deficit;
    u16 resident_count;
    u16 candidate_count;
    u16 candidate;

    result.status = -1;
    result.destination = (volatile u8 FAR *)0;
    current_handle = handle;
    current_entry_offset = (u16)(handle * 8u);
    entry = &entries[handle];
    if (entry->flags & 3u) {
        entry->flags |= 2u;
        result.status = 1;
        result.destination = (volatile u8 FAR *)MK_FP(entry->segment, 0u);
        return result;
    }

    rounded = (byte_count + 15u) & 0xfffffff0UL;
    if ((i32)rounded > (i32)free_bytes) {
        resident_count = 0u;
        while (resident_count < 256u && residents[resident_count] != 0xffffu) {
            ++resident_count;
        }
        deficit = (i32)(rounded - free_bytes);
        candidate_count = 0u;
        while (resident_count != 0u) {
            --resident_count;
            candidate = residents[resident_count];
            candidate_entry = &entries[candidate];
            if ((candidate_entry->flags & 2u) == 0) {
                evictions[candidate_count++] = candidate;
                deficit -= candidate_entry->size;
                if ((i32)deficit <= 0) {
                    break;
                }
            }
        }
        evictions[candidate_count] = 0xffffu;
        if ((i32)deficit >= 0) {
            failure_probe(2u);
            return result;
        }
        candidate_count = 0u;
        while ((i16)evictions[candidate_count] >= 0) {
            free_probe(evictions[candidate_count++]);
        }
    }

    entry = &entries[handle];
    entry->segment = pool_end_segment;
    entry->flags |= 3u;
    entry->size = rounded;
    free_bytes -= rounded;
    pool_end_segment = (u16)(pool_end_segment + (u16)(rounded >> 4));
    resident_count = 0u;
    while (resident_count < 256u && residents[resident_count] != 0xffffu) {
        ++resident_count;
    }
    residents[resident_count] = handle;
    residents[resident_count + 1u] = 0xffffu;
    result.destination = (volatile u8 FAR *)MK_FP(entry->segment, 0u);
    if (entry->flags & 0x000cu) {
        special_entry = (volatile resource_entry FAR *)MK_FP(
                entry->segment, (u16)(handle * 8u));
        special_entry->flags |= 2u;
        result.destination = (volatile u8 FAR *)MK_FP(special_entry->segment, 0u);
        result.status = 1;
    } else {
        result.status = 0;
    }
    return result;
}
