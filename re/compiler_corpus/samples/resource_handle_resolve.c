/* Codegen probe for BLOODPRG 0x005320. */
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

typedef struct resource_resolve_result_probe {
    u16 segment;
    u16 offset;
    u16 loaded;
} resource_resolve_result_probe;

extern volatile resource_handle_probe resource_handle_table_probe[];

resource_resolve_result_probe FAR resource_handle_resolve_probe(u16 handle)
{
    resource_resolve_result_probe result;
    const volatile resource_handle_probe *entry;

    result.segment = 0;
    result.offset = 0;
    result.loaded = 0;

    entry = &resource_handle_table_probe[handle];
    if (entry->flags & 3u) {
        result.segment = entry->segment;
        result.loaded = 1;
    }

    return result;
}
