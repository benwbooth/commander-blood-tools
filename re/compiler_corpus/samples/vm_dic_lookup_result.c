/*
 * Codegen probe for BLOODPRG 0x006433.
 * This is not recovered game source.
 */
typedef unsigned int u16;

#if defined(__TURBOC__) || defined(__BORLANDC__) || defined(__WATCOMC__)
#define FAR far
#define NEAR near
#else
#define FAR
#define NEAR
#endif

#define DIRECTORY_ACTIVE_KIND 0x0001u

typedef struct dir_entry {
    char name[16];
    u16 object_offset;
    u16 entry_kind;
} dir_entry;

typedef struct lookup_result {
    u16 object_offset;
    int matched;
} lookup_result;

extern const char FAR *dic_words;
extern const volatile dir_entry FAR *record_directory;
int FAR string_equal_probe(const volatile char FAR *left,
        const volatile char FAR *right);

lookup_result NEAR vm_dic_lookup_result_probe(u16 dictionary_offset)
{
    const volatile char FAR *word;
    const volatile dir_entry FAR *entry;
    lookup_result result;

    word = dic_words + dictionary_offset;
    entry = record_directory;
    while (entry->entry_kind == DIRECTORY_ACTIVE_KIND) {
        if (string_equal_probe(word, entry->name)) {
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
