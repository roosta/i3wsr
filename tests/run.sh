#!/usr/bin/env bash
# tests/run.sh — headless X + i3, run the test suite, tear everything down on exit.
set -euo pipefail

cd "$(dirname "$0")/.."

LOG=${LOG:-/tmp/xsession.log}
export LIBGL_ALWAYS_SOFTWARE=1 NO_AT_BRIDGE=1   # quieter mesa / at-spi

pids=()
rundir=''
cleanup() {
  local p
  for p in "${pids[@]}"; do kill "$p" 2>/dev/null || true; done
  rm -rf "$rundir"
  return 0
}
trap cleanup EXIT INT TERM

wait_for() {                        # poll until "$@" succeeds, or WAIT_TIMEOUT (default 15s)
  local deadline=$(( SECONDS + ${WAIT_TIMEOUT:-15} )) rc
  until "$@" >/dev/null 2>&1; do
    rc=$?
    if (( rc == 126 || rc == 127 )); then
      printf 'wait_for: cannot execute %s\n' "$1" >&2
      return 1
    fi
    if (( SECONDS >= deadline )); then
      printf 'timed out waiting for: %s\n' "$*" >&2
      [ -s "$LOG" ] && { printf -- '--- %s ---\n' "$LOG" >&2; tail -n 20 "$LOG" >&2; }
      return 1
    fi
    sleep 0.1
  done
}

i3_has_class() { i3-msg -t get_tree | grep -q "\"class\":\"$1\""; }

# --- X server: -displayfd signals readiness, X picks a free display ---------
rundir=$(mktemp -d)
fifo=$rundir/display
mkfifo "$fifo"

: >"$LOG"
Xvfb -displayfd 3 -screen 0 1280x1024x24 -nolisten tcp -noreset \
  2>>"$LOG" 3>"$fifo" &
pids+=($!)

dump_log() {
  [ -s "$LOG" ] && { printf -- '--- %s (tail) ---\n' "$LOG" >&2; tail -n 40 "$LOG" >&2; }
  return 0
}

if ! read -t "${WAIT_TIMEOUT:-15}" -r dispnum < "$fifo"; then
  printf 'Xvfb did not come up\n' >&2
  dump_log
  exit 1
fi
export DISPLAY=":$dispnum"

# --- window manager ---------------------------------------------------------
i3 -c /dev/null >>"$LOG" 2>&1 &
pids+=($!)
wait_for i3-msg -t get_version               # WM up (distinct from "windows mapped")

# --- the windows i3wsr should see -------------------------------------------
gpick >>"$LOG" 2>&1 &
pids+=($!)
xterm >>"$LOG" 2>&1 &
pids+=($!)

wait_for i3_has_class XTerm
wait_for i3_has_class Gpick
i3-msg '[class="XTerm"] floating enable' >>"$LOG" 2>&1

# --- tests ------------------------------------------------------------------
if ! cargo test "$@"; then
  printf -- '--- %s (tail) ---\n' "$LOG" >&2
  tail -n 40 "$LOG" >&2
  exit 1
fi
