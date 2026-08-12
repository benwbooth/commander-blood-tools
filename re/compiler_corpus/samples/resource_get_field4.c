/* Codegen probe for BLOODPRG 0x00533C. */
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
    u32 field_04;
} resource_handle_probe;

extern volatile resource_handle_probe resource_handle_table_probe[];

#if defined(__WATCOMC__)
#pragma aux resource_get_field4_probe parm [ax] value [dx ax] modify exact [ax dx]
#endif

u32 FAR resource_get_field4_probe(u16 handle)
{
    return resource_handle_table_probe[handle].field_04;
}
