/* Codegen probe for BLOODPRG 0x00A20C. */
typedef unsigned short u16;

#if defined(__WATCOMC__)
#define FAR __far
#define NEAR __near
#elif defined(__TURBOC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

extern volatile u16 resource_flags;
extern volatile u16 list_d8c_active_segment;
extern volatile u16 list_d8c_byte_count;
extern volatile u16 FAR *list_d8c_tail_pointer;
extern volatile u16 list_d8c_default_entry_segment;
extern volatile u16 list_d8c_alternate_entry_segment;

extern void NEAR list_d8c_activate_entry_probe(u16 entry_extent,
        volatile u16 FAR *entry, u16 storage_segment);

int NEAR list_d8c_activate_ready_probe(void)
{
    volatile u16 FAR *entry;
    u16 entry_extent;
    u16 storage_segment;
    u16 queued_bytes;

    if (list_d8c_active_segment != 0) {
        return 1;
    }

    queued_bytes = list_d8c_byte_count;
    if (queued_bytes == 0) {
        return 0;
    }

    entry = list_d8c_tail_pointer;
    entry_extent = *entry++;
    if (*entry != 0x6d6du && queued_bytes < entry_extent) {
        return 0;
    }

    storage_segment = list_d8c_default_entry_segment;
    if ((resource_flags & 0x0040u) != 0) {
        storage_segment = list_d8c_alternate_entry_segment;
    }
    list_d8c_activate_entry_probe(entry_extent, entry, storage_segment);
    return 1;
}
