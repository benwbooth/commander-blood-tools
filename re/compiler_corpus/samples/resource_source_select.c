/*
 * Codegen probe for BLOODPRG 0x002693.
 * This is not recovered game source.
 */
typedef unsigned char u8;
typedef unsigned int u16;

#if defined(__WATCOMC__)
#define GAME_DATA __based(__segname("GAME_DATA"))
#else
#define GAME_DATA far
#endif

typedef struct resource_name_entry_probe {
    char filename[16];
} resource_name_entry_probe;

extern const resource_name_entry_probe GAME_DATA write_directory_names_probe[];
extern volatile u8 GAME_DATA force_write_directory_probe;
extern volatile u8 GAME_DATA path_is_embedded_probe;

int far string_equal_probe(const volatile char *left,
        const volatile char far *right);
void far write_directory_enter_probe(void);
void far original_directory_restore_probe(void);
u16 near archive_match_probe(const volatile char *filename);

#if defined(__WATCOMC__)
#pragma aux string_equal_probe parm [si] [es di] value [ax] modify exact [ax]
#pragma aux archive_match_probe parm [dx] value [bx] modify [ax bx]
#pragma aux resource_source_select_probe parm [dx] value [bx] modify [ax bx cx dx]
#endif

u16 far resource_source_select_probe(const volatile char *filename)
{
    const resource_name_entry_probe GAME_DATA *entry;

    path_is_embedded_probe = 0;
    if ((force_write_directory_probe & 1u) != 0u) {
        write_directory_enter_probe();
        return 0;
    }

    entry = write_directory_names_probe;
    do {
        if (string_equal_probe(filename, entry->filename)) {
            write_directory_enter_probe();
            return 0;
        }
        ++entry;
    } while (entry->filename[0] != '\0');

    original_directory_restore_probe();
    return archive_match_probe(filename);
}
