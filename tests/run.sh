#!/usr/bin/env bash
# tests/run.sh — headless X + i3, run the test suite, tear everything down on exit.
set -euo pipefail

cd "$(dirname "$0")/.."

LOG=${LOG:-$(mktemp)}

pids=()
rundir=''
cleanup() {
  local p
  for p in "${pids[@]}"; do kill "$p" 2>/dev/null || true; done
  [ -n "$rundir" ] && rm -rf "$rundir"
  return 0
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

i3_classes() { i3-msg -t get_tree | grep -o '"class":"[^"]*"' | sort -u; }

dump_log() {
  [ -s "$LOG" ] && { printf -- '--- %s (tail) ---\n' "$LOG" >&2; tail -n 40 "$LOG" >&2; }
  if [ -n "${DISPLAY:-}" ]; then
    printf -- '--- window classes i3 knows about ---\n' >&2
    i3_classes >&2 || true
  fi
  return 0
}

# poll until "$@" succeeds, or WAIT_TIMEOUT (default 15s)
wait_for() {
  local deadline=$(( SECONDS + ${WAIT_TIMEOUT:-15} )) rc
  until "$@" >/dev/null 2>&1; do
    rc=$?
    if (( rc == 126 || rc == 127 )); then
      printf 'wait_for: cannot execute %s\n' "$1" >&2
      return 1
    fi
    if (( SECONDS >= deadline )); then
      printf 'timed out waiting for: %s\n' "$*" >&2
      dump_log
      return 1
    fi
    sleep 0.1
  done
}

i3_has_class() { i3-msg -t get_tree | grep "\"class\":\"$1\"" >/dev/null; }

# X server: -displayfd signals readiness, X picks a free display
rundir=$(mktemp -d)
fifo=$rundir/display
mkfifo "$fifo"

: >"$LOG"
Xvfb -displayfd 3 -screen 0 1280x1024x24 -nolisten tcp -noreset \
  2>>"$LOG" 3>"$fifo" &
pids+=($!)

if ! read -t "${WAIT_TIMEOUT:-15}" -r dispnum < "$fifo"; then
  printf 'Xvfb did not come up\n' >&2
  dump_log
  exit 1
fi
export DISPLAY=":$dispnum"

# window manager
i3 -c /dev/null >>"$LOG" 2>&1 &
pids+=($!)
wait_for i3-msg -t get_version

# window fixtures: two xterms whose class/instance i3wsr should read
xterm -name xterm -class XTerm >>"$LOG" 2>&1 &
pids+=($!)
xterm -name emacs -class Emacs >>"$LOG" 2>&1 &
pids+=($!)

wait_for i3_has_class XTerm
wait_for i3_has_class Emacs
i3-msg '[class="XTerm"] floating enable' >>"$LOG" 2>&1

# tests
if ! cargo test "$@"; then
  dump_log
  exit 1
fi
