/*
 * Codegen probe for BLOODPRG 0x009F80.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define NEAR near
#else
#define NEAR
#endif

typedef struct resource_descriptor_probe {
    u8 flags;
    u8 variant;
    char filename[1];
} resource_descriptor_probe;

typedef struct resource_index_entry_probe {
    resource_descriptor_probe NEAR *descriptor;
    u16 secondary_resource_id;
} resource_index_entry_probe;

extern volatile resource_index_entry_probe resource_index_probe[];

#if defined(__WATCOMC__)
#pragma aux resource_descriptor_lookup_probe parm [ax] value [bx] modify [bx]
#endif

resource_descriptor_probe NEAR *NEAR resource_descriptor_lookup_probe(u16 index)
{
    return resource_index_probe[index].descriptor;
}
