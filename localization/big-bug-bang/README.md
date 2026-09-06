# Big Bug Bang English Display Text

`en/script1.json` contains an English editorial first pass for all 89 A6 text
sites in the opening COD profile. It includes non-spoken sites, unchanged sound
effects, and the three choice sections. The COD runtime backend now binds its
subtitle sections, inline menu prose, and choice labels for the matching opening
profile. The normal game loader now
accepts the verified BBB build, but complete gameplay is not established, so this
is not yet a playable English release.
`en/script2.json` now supplies all 1,197 text sites in the second COD profile
(985 unique sections), including 11 live-number menus. The modern runtime binds
this catalog to matching SCRIPT2 resources. Source-aware validation and runtime
catalog tests pass. The ordinary startup/PLAY capture
`output/big-bug-bang/english-script2-play-02` reaches profile 1 and visibly renders
"Go and look in the cryobox. Old Daddy is waiting for you there..."
(`screen-028.png`). This verifies the first SCRIPT2 subtitle, not every site.
The subsequent TV sequence still displays French text; contextual review and
the remaining localization layers are unfinished.
The integration run passed six localization tests with original resources,
game-package all-targets checking, and 948 serial game-library tests (31 ignored).
Workspace-wide all-targets checking failed in the script-compiler test target
on unresolved shared-module imports; it is not a passing workspace gate.
`en/script3.json` supplies all 779 text sites in the third profile (663 unique
sections). It is bound to matching resources, including three live population
readouts, four inventory prompts, and one intentionally empty text site.
Inventory sections preserve their generator marker and receive no static choice
override: the live inventory remains authoritative. Item-name localization is
still separate work. All eight localization tests pass with original resources;
SCRIPT3 live English rendering and contextual review remain unverified.
Game-package all-targets checking and 949 serial game-library tests pass
(32 ignored) after this integration.
The other 14 COD profiles, BAS text, native UI, object names, and text embedded
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

The runtime menu path now parses standalone `<state:N>` words into typed live
number references. It requires the same ordered references as the original
prose section, rejecting missing, added, reordered, or malformed markers. The
renderer reads each reached number from current VAR state as a signed 16-bit
value. Lookahead retains the native previous-number scratch value instead of
reading the next number early. Original words, word counts, and VAR are not
rewritten. Numeric overrides are restricted to non-spoken menu text; inventory
generators in the sole post-prose choice section remain live while their prompt
is translated. Mixed generator/static choice sections are not supported.

This support is used by the 11 numeric menu sites in SCRIPT2. Tests cover
signed limits, live changes, state preservation, marker-source validation, and
the existing original numeric-renderer vectors.

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
the modern runtime binds English catalogs for BBB SCRIPT1, SCRIPT2, and SCRIPT3
with matching COD and DIC SHA-256 hashes. Other profiles, modified resources,
and missing sites retain their original text. Binding another profile clears
the old translation. Original dictionary IDs and menu words are never replaced.

The retained choice renderer uses the last accepted A6 instruction as its display
source. English labels are bound to that instruction and the exact ordered source
dictionary IDs; missing sites, reordered choices, inventory lists, and unmatched
resources keep their original labels. Rejected A6 calls do not change the source,
and a profile reset clears it. Width measurement and drawing use the translated
labels, while choice completion retains the original dictionary identity.

There are 34 authored spoken-flag sites in the opening profile. Inline menu prose
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
of reaching Honk from a new game. The earlier ordinary-pointer PLAY run
`output/big-bug-bang/modern-honk-play-02` failed during the shared-VAR transition
at byte 8368. That loader rejection is now resolved using the native-captured
read-only SCRIPT2.DEB prefix binding, without extending VAR. The repeated run
`modern-honk-play-04` loads SCRIPT2 immediately after PLAY and reaches new French
dialogue. Translation bindings correctly stop at the profile boundary rather
than applying SCRIPT1 text to SCRIPT2 addresses. Starting SCRIPT2 without the
preceding persistent state is still rejected. The attempted cryobox sequence
has not yet reached Bob, so its translation remains without a live visual check.

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
