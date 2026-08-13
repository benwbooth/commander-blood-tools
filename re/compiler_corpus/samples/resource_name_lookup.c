/*
 * Codegen probe for BLOODPRG 0x0028CA.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned long u32;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

typedef struct dos_dta_probe {
    u8 reserved_00[0x1a];
    u32 file_size;
} dos_dta_probe;

extern volatile u32 archive_remaining_probe;
extern volatile u8 path_is_embedded_probe;

unsigned int FAR resource_source_select_probe(volatile char FAR *filename);
volatile dos_dta_probe FAR *NEAR dos_get_dta_probe(void);
int NEAR dos_find_first_probe(const volatile char FAR *filename);

u32 FAR resource_name_lookup_probe(volatile char FAR *filename)
{
    volatile dos_dta_probe FAR *dta;
    u32 byte_count;

    (void)resource_source_select_probe(filename);
    byte_count = archive_remaining_probe;
    if ((path_is_embedded_probe & 1u) == 0) {
        dta = dos_get_dta_probe();
        (void)dos_find_first_probe(filename);
        byte_count = dta->file_size;
    }

    return byte_count;
}
