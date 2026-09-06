# Big Bug Bang English Display Text

`en/script1.json` contains an English editorial first pass for all 89 A6 text
sites in the opening COD profile. It includes non-spoken sites, unchanged sound
effects, and the three choice sections. The COD runtime backend now binds its
subtitle sections, inline menu prose, and choice labels for the matching opening
profile. The normal game loader now
accepts the verified BBB build, but complete gameplay is not established, so this
is not yet a playable English release.
The other 16 COD profiles, BAS text, native UI, object names, and text embedded
in media remain untranslated.

Validate against the user's original resources:

```sh
nix develop -c cargo run --bin sequel_text_catalog -- \
  output/big-bug-bang/imported-assets/resources \
  --validate localization/big-bug-bang/en/script1.json
```

Each message key retains its COD instruction address. Array elements correspond
to the source catalog's sections in order. The first is prose; later sections
retain one space-separated display label per original choice in its original
position. `PLAY INSTRUCTIONS` labels `JOUER EXPLICATIONS`, without replacing the
underlying dictionary IDs. Never use English display words for conditions,
history, audio hashes, or dictionary lookups. Source COD and DIC hashes bind the
file to the specific authored resources, not to a similar-looking script.

V1 uses printable ASCII for the existing font path. Dynamic markers retain
their original order and spelling (`<state:N>` and `<inventory_choices>`).
The validator checks structural compatibility, not translation quality, choice
meaning, rendered width, reachability, or gameplay.

Editorial choices: Monsieur Bob becomes Mr. Bob; Biorédactrice becomes
bio-editor; proper names and invented terms such as GLUXX and BIONIUM remain.
The deliberately split `A DIEU` becomes `TO GOD` to preserve the farewell pun.
Bob's unusual phrase at `00000c2c` is translated literally pending contextual
review with the recording; it has not been silently corrected into a different
French sentence. English voice acting is not supplied.

## Subtitle and Menu Integration

The COD dispatcher requests a display override only after `SubtitlePublished`.
The backend substitutes section zero, wrapped at 34 columns with the existing
carriage-return line format. Native/reference hosts default to original text;
the modern runtime binds the English catalog only for the initial BBB profile
with matching COD and DIC SHA-256 hashes. Other profiles, modified resources,
and missing sites retain their original text. Binding another profile clears
the old translation. Original dictionary IDs and menu words are never replaced.

The retained choice renderer uses the last accepted A6 instruction as its display
source. English labels are bound to that instruction and the exact ordered source
dictionary IDs; missing sites, reordered choices, inventory lists, and unmatched
resources keep their original labels. Rejected A6 calls do not change the source,
and a profile reset clears it. Width measurement and drawing use the translated
labels, while choice completion retains the original dictionary identity.

There are 34 authored spoken-flag sites in this profile. Inline menu prose
(including Bob's recording and OLGA's dialogue) now uses English display words
when its complete authored word stream matches the accepted instruction. The
shared menu layout measures and wraps those words and completes at their own
word count. Authored words, number operands, dictionary IDs, and chatter inputs
remain untouched. Inventory and unmatched streams retain their original text.
The trace keeps original `words`/`word_ids` separately from `display_words`;
localized revealed words have null dictionary IDs rather than invented ones.

The three dictionary-choice sections are bound to English display labels. Only
the opening HONK menu has been reached and visually checked in the live runtime;
no live Bob/OLGA menu-prose reachability is claimed.

Verification commands:

```sh
nix develop -c cargo test -p commander-blood-game --lib runtime::localization -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib english_subtitle -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib english_inline_menu_binding -- --include-ignored
nix develop -c cargo test -p commander-blood-game --lib menu_reveal -- --include-ignored
```

These exercise source binding, hash-mismatch fallback, original-font rasterization
of all 89 translated prose sections, and reveal completion at the translated
length. An isolated call through the real A6 dispatcher compares original and
English outputs: only subtitle bytes differ; selector, choice, VAR, VM, and other
dispatch state remain identical. Gated and menu-only calls do not invoke the
subtitle hook; accepted menu prose binds separately at the renderer. All 89
English prose sections also pass complete inline-font raster and screen-bound
checks. A focused reveal test verifies translated completion timing and unchanged
source IDs; an original-resource binding test checks stale-stream/profile fallback.
The fixture explicitly prepares the actor-presentation gate; it is not evidence
of reaching Honk from a new game. Actual cross-profile verification remains
blocked: the SCRIPT1 -> SCRIPT2 shared-VAR transition fails at byte 8368, and
starting SCRIPT2 without that preceding state is rejected as missing persistent
state. These tests do not bypass either constraint. The ordinary-pointer PLAY
run `output/big-bug-bang/modern-honk-play-02` now confirms that the same state-load
error occurs after real menu selection and the opening English dialogue.

The game library regression suite passed 940 tests (25 ignored) with
`--test-threads=1`. A parallel rerun terminated with SIGSEGV; its core dump placed
the crashing stack in Vulkan `loader_get_icd_and_device` /
`SetDebugUtilsObjectNameEXT` while
`render::tests::srgb_artwork_and_overlay_match_every_cpu_expanded_dac_level`
created a pipeline layout. The root cause is not established or fixed here;
the serial pass does not prove parallel GPU-test stability. Workspace checking
and the five targeted localization/dispatch checks passed.

Remaining work includes BAS/UI translation,
the other COD profiles, contextual editorial review, and actual playable startup
and progression. No English voice acting or whole-game localization is claimed.

The menu-prose integration regression run passed 942 library tests serially
(29 ignored), all three menu-reveal checks, four localization checks, and the
original-resource runtime binding test. All-targets checking passed. These do
not establish complete gameplay or live reachability of every translated menu.
