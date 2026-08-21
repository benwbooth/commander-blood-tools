#include "../include/bloodprg_vm.h"

bloodprg_dic_lookup_result CB_NEAR dic_word_lookup(cb_u16 dictionary_offset)
{
    const volatile char CB_FAR *word;
    const volatile bloodprg_vm_directory_entry CB_FAR *entry;
    bloodprg_dic_lookup_result result;

    word = vm_dic_words_gs + dictionary_offset;
    entry = vm_record_directory_gs;

    while (entry->entry_kind == BLOODPRG_VM_DIRECTORY_ACTIVE_KIND) {
        if (string_compare(word, entry->name)) {
            result.object_offset = entry->object_offset;
            result.matched = 1;
            return result;
        }
        ++entry;
    }

    result.object_offset = entry->object_offset;
    result.matched = 0;
    return result;
}
