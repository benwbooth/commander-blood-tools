# Natural C Candidate Coverage

Current measured coverage:

| module | indexed routines | natural-C candidates | missing |
| --- | ---: | ---: | ---: |
| `bloodprg` | 321 | 321 | 0 |
| `xdb_amer` | 54 | 54 | 0 |
| `xdb_croolis` | 47 | 47 | 0 |
| `xdb_manu3` | 12 | 12 | 0 |
| `xdb_scrut` | 37 | 37 | 0 |
| total | 471 | 471 | 0 |

Overall candidate coverage is 471 of 471 standardized indexed routines, or
100.00 percent. One additional legacy XDB candidate dump is deliberately
reported as unindexed: the overlapping SCRUT callback-state range still needs
a control-flow ownership audit before it can be split into routine assembly
owners.

Generate the live report, including the missing routine list, with:

```sh
python3 re/tools/source_candidates.py --coverage
```

The JSON report is derived from `re/assembly/routine_index.tsv` plus every
`re/source/**/candidates/manifest.tsv`. A routine is counted as covered only
when the candidate manifest entry resolves to the same module and routine
offset as the assembly inventory.
