#!/usr/bin/env bash
# RE loop: run Claude sessions until all Next Tasks in re/REVERSE.md are done.
# Usage: ./re/re_loop.sh [--max N] [--tasks N] [--max-turns N] [--dry-run]
# Run from the REPO ROOT (tools resolve as re/tools/* under the nix dev shell).
set -euo pipefail

# ── PROJECT CONFIG ────────────────────────────────────────────────────────
GAME="Commander Blood (BLOODPRG.EXE)"
BINARY="re/bin/BLOODPRG.EXE"
TOOLS_PREFIX="nix develop --command python3 re/tools/"
LABELS="re/labels.csv"
DEAD_ENDS="re/dead_ends.md"
REVERSE="re/REVERSE.md"
MAX_TURNS=150
GIT_ADD="re/REVERSE.md re/labels.csv re/dead_ends.md re/tools/ re/CLAUDE.md docs/ src/"
ALLOWED_TOOLS="\
Bash(nix develop --command python3 re/tools/*),\
Bash(nix develop --command cargo*),\
Bash(git add*),Bash(git commit*),\
Bash(git log*),Bash(git status*),Bash(git diff*),\
Read,Edit,Write,Glob,Grep"
# ──────────────────────────────────────────────────────────────────────────

cleanup() { echo ""; echo "Interrupted — killing session..."; kill %1 2>/dev/null || true; exit 130; }
trap cleanup INT TERM

MAX=50; TASKS=1; DRY=false
while [[ $# -gt 0 ]]; do
  case $1 in
    --max)       MAX="$2";       shift 2 ;;
    --tasks)     TASKS="$2";     shift 2 ;;
    --max-turns) MAX_TURNS="$2"; shift 2 ;;
    --dry-run)   DRY=true;       shift ;;
    *) echo "unknown: $1"; exit 1 ;;
  esac
done

cd "$(dirname "$0")/.."   # repo root
remaining() { grep -c '^- \[ \]' "$REVERSE" 2>/dev/null || true; }
mkdir -p re/re_loop_sessions
RUN_TS=$(date '+%Y%m%d_%H%M%S')

for (( i=1; i<=MAX; i++ )); do
  [[ $(remaining) -eq 0 ]] && echo "All tasks done!" && break
  echo ""; echo "=== Session $i ($(remaining) tasks left) ==="
  [[ "$DRY" == true ]] && echo "[dry-run]" && break
  LOG="re/re_loop_sessions/${RUN_TS}_session_$(printf '%03d' $i).txt"

  PROMPT="Continue the $GAME reverse-engineering project. Goal: recover the
script-VM + presentation semantics needed to generate game-accurate cutscene
videos, then drive an event-based renderer in the Rust crate (src/extract/).

Before picking a task:
1. Read $DEAD_ENDS to avoid known dead ends.
2. Read re/CLAUDE.md for conventions, then $REVERSE ## Next Tasks.
3. Skim the last 3 logs in re/re_loop_sessions/.

Pick the top $TASKS unchecked RE-investigation item (- [ ]). Then:
- Run 2-3 tool calls (${TOOLS_PREFIX}dis.py / search_bytes.py / xref.py /
  seg_offset.py / strings_dump.py / dump_*.py).
- Write findings to $REVERSE immediately; add addresses to $LABELS.
- Repeat until the task is understood. Do NOT batch all investigation first.

If stuck after 10 tool calls with no progress: append to $DEAD_ENDS
(tried / why-failed / better-approach / session), split the task into 2-3
sub-tasks in $REVERSE, mark original [x] 'Split into sub-tasks', move on.

Tool rules: Read (not cat), Grep (not grep), Glob (not ls/find). Write a .py
file under re/tools/ first — never inline python3 -c. capstone via dis.py is OK.

Mark the task [x] once documented. End with: SESSION_SUMMARY: <one line>.
Stop after $TASKS task(s)."

  echo "$PROMPT" | claude -p --output-format stream-json --max-turns "$MAX_TURNS" \
    --allowedTools "$ALLOWED_TOOLS" \
    | jq --unbuffered -r '
        if .type == "assistant" then .message.content[] |
          if .type == "text" then .text
          elif .type == "tool_use" then "  ▶ \(.name) \(.input.command // .input.file_path // (.input|keys|join(" ")) | tostring | .[0:120])"
          else empty end
        else empty end' | tee "$LOG" &
  wait $!

  SUMMARY=$(git diff "$REVERSE" | grep '^+- \[x\]' | head -1 | sed 's/^+- \[x\] //' || true)
  [[ -z "$SUMMARY" ]] && SUMMARY="session $i progress"
  git add $GIT_ADD 2>/dev/null || true
  if git diff --cached --quiet; then echo "No changes — retrying..."; continue; fi
  git commit -m "RE session $i: $SUMMARY"
  echo "Committed: $SUMMARY"
done
echo ""; echo "Done. Remaining tasks: $(remaining)"
