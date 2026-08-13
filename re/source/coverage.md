# Natural C Candidate Coverage

Current measured coverage:

| module | indexed routines | natural-C candidates | missing |
| --- | ---: | ---: | ---: |
| `bloodprg` | 318 | 149 | 169 |
| `xdb_amer` | 27 | 17 | 10 |
| `xdb_croolis` | 25 | 15 | 10 |
| `xdb_manu3` | 12 | 12 | 0 |
| `xdb_scrut` | 25 | 15 | 10 |
| total | 407 | 208 | 199 |

Overall candidate coverage is 208 of 407 indexed routines, or 51.11 percent.
All current candidates point at indexed routines.

Generate the live report, including the missing routine list, with:

```sh
python3 re/tools/source_candidates.py --coverage
```

The JSON report is derived from `re/assembly/routine_index.tsv` plus every
`re/source/**/candidates/manifest.tsv`. A routine is counted as covered only
when the candidate manifest entry resolves to the same module and routine
offset as the assembly inventory.
