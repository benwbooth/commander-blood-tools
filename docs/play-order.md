# Play-order masters

`tools/play_order.py` derives a static play order for every text site and HNM video
of one game from the recovered BloodScript (`static-dialogue-en-v2/<game>/scriptN/source.blood`,
which recompiles byte-exact to the original COD/BAS). `tools/play_order_master.py`
joins the existing renders in that order into one chaptered master per game.

## How the order is derived

- Profiles start at SCRIPT1 and follow `run profile` statements in the order they become
  reachable (CB: SCRIPT1-5; BBB: SCRIPT1, the SCRIPT2 hub, then the episodes).
- Each profile is interpreted in rounds over value sets: the state block and `global`
  declarations seed the variables; assignments, `+=`/`-=`, `bits(...) =` and `transfer`
  grow them. Player input (`choice`, `presentation`, `during`), engine-owned fields
  (current location, holder, known, encounter counts) and globals that a guard tests for
  a value no script can produce (the sequel engine writes those) may take any value.
  A proc fires in the first round where it is enabled and its guards can hold; a `when`
  that needs later effects is reached in a later round (a later visit).
- Globals carry across profiles, and whole passes repeat until they stop growing, so
  hub content reached after an episode plays in a later visit.
- Within a round, procs that change the game play before procs that only show text
  (console menus), then source order.
- An on-planet (`during travel`) conversation follows the first line by another actor
  that names its planet or location: the player cannot fly somewhere unknown.
- Videos sit where the script first shows them: `present` opens the game (engine's
  initial sequence slot), sequence records at `sequence_slots[n] = "record"`, `request
  sequence` resources and `say ... presentation=clip.hnm` clips at their statement,
  location videos before the first on-planet line there, object videos at the first
  transfer aboard, remaining character clips at the actor's first contact.

## Assembly

Per entry: the verified site render from the static-text ledger; the native capture of
a whole sequence record (its videos are then covered); otherwise the video's chapter of
the all-HNM master, cut frame-exact and re-encoded losslessly. A clip whose line already
renders as a verified site is not repeated. Videos no script names (hyperspace, cryobox,
logos, TV channels, unused clips) follow the game as a final block.

```sh
A=output/anthology
PYTHONSAFEPATH=1 PYTHONPATH=tools python3 tools/play_order.py --dialogue $A/static-dialogue-en-v2 \
  --inventory $A/static-content-v1/inventory.json --catalog $A/catalog-cb.json --game cb \
  --out $A/play-order-v1/cb.json
nix develop -c env PYTHONSAFEPATH=1 PYTHONPATH=tools python3 tools/play_order_master.py \
  --order $A/play-order-v1/cb.json --ledger $A/static-text-batches/ledger-v4.json \
  --hnm $A/all-hnm-cb-v5 --sequences $A/native-sequences-v2/cb \
  --inventory $A/static-content-v1/inventory.json --out $A/play-order-v1/cb-master
```

## Results (2026-11-12, ledger v4, all-HNM v5)

| Master | Hours | Sites | Sequence records | Videos | Engine-played block |
| --- | ---: | ---: | ---: | ---: | ---: |
| `play-order-v1/cb-master/cb-play-order.mkv` | 5.19 | 5,500 | 10 | 280 | 127 |
| `play-order-v1/bbb-master/bbb-play-order.mkv` | 5.77 | 6,874 | 41 | 463 | 144 |

Limits: the order is a static derivation, not a recorded playthrough; player-elective
contacts in the same round keep source order. 90 CB and 386 BBB sites sit behind guards
the analysis cannot satisfy and play at the end of their profile's last visit. 36 CB and
47 BBB sites still have no verified render. HNM chapters without a native clip render
use the decoder's context-free palette, so some flybys and entrance clips show wrong
colours; those come from the all-HNM master and need native renders there.
