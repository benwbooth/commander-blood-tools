# Natural C Candidate Coverage

Current measured coverage:

| module | indexed routines | natural-C candidates | missing |
| --- | ---: | ---: | ---: |
| `bloodprg` | 321 | 321 | 0 |
| `xdb_amer` | 38 | 38 | 0 |
| `xdb_croolis` | 36 | 36 | 0 |
| `xdb_manu3` | 12 | 12 | 0 |
| `xdb_scrut` | 36 | 36 | 0 |
| total | 443 | 443 | 0 |

Overall candidate coverage is 443 of 443 standardized indexed routines, or
100.00 percent. Thirteen additional legacy XDB candidate dumps are deliberately
reported as unindexed: their broad callback-state ranges still need a split
audit before they can be promoted to one-routine assembly owners.

Generate the live report, including the missing routine list, with:

```sh
python3 re/tools/source_candidates.py --coverage
```

The JSON report is derived from `re/assembly/routine_index.tsv` plus every
`re/source/**/candidates/manifest.tsv`. A routine is counted as covered only
when the candidate manifest entry resolves to the same module and routine
offset as the assembly inventory.
