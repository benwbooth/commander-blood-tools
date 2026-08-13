#include "../include/bloodprg_resource.h"
#include "../include/bloodprg_startup.h"
#include "../include/bloodprg_vm.h"

cb_u16 CB_FAR resource_source_select(volatile char CB_FAR *filename)
{
    const bloodprg_resource_name_entry CB_GAME_DATA *entry;

    resource_path_is_embedded = 0;
    if ((resource_force_write_directory & 1u) != 0u) {
        startup_write_directory_enter();
        return 0;
    }

    entry = resource_write_directory_names;
    do {
        if (string_compare(filename, entry->filename)) {
            startup_write_directory_enter();
            return 0;
        }
        ++entry;
    } while (entry->filename[0] != '\0');

    startup_original_directory_restore();
    return resource_archive_match(filename);
}
