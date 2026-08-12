# Natural C Candidate Coverage

Current measured coverage:

| module | indexed routines | natural-C candidates | missing |
| --- | ---: | ---: | ---: |
| `bloodprg` | 318 | 139 | 179 |
| `xdb_amer` | 25 | 3 | 22 |
| `xdb_croolis` | 25 | 3 | 22 |
| `xdb_manu3` | 18 | 1 | 17 |
| `xdb_scrut` | 25 | 3 | 22 |
| total | 411 | 149 | 262 |

Overall candidate coverage is 149 of 411 indexed routines, or 36.25 percent.
All current candidates point at indexed routines.

Generate the live report, including the missing routine list, with:

```sh
python3 re/tools/source_candidates.py --coverage
```

The JSON report is derived from `re/assembly/routine_index.tsv` plus every
`re/source/**/candidates/manifest.tsv`. A routine is counted as covered only
when the candidate manifest entry resolves to the same module and routine
offset as the assembly inventory.
