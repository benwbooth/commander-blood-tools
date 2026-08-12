/* Codegen probe for BLOODPRG 0x005288. */
typedef unsigned int u16;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#else
#define FAR
#endif

typedef struct resource_handle_probe {
    u16 segment;
    u16 flags;
    u32 size;
} resource_handle_probe;

extern const volatile resource_handle_probe resource_handle_table_probe[];
void FAR resource_free_inner_probe(u16 handle);

#if defined(__WATCOMC__)
#pragma aux resource_release_probe parm [ax] modify exact []
#pragma aux resource_free_inner_probe parm [ax] modify exact []
#endif

void FAR resource_release_probe(u16 handle)
{
    const volatile resource_handle_probe *entry;

    entry = &resource_handle_table_probe[handle];
    if (entry->flags & 3u) {
        resource_free_inner_probe(handle);
    }
}
