#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source="$root/src/main.rs"

if ! awk '
  /connect_activate/ { in_activate = 1 }
  in_activate && /if let Some\(existing\)/ { guarded = 1 }
  in_activate && /build_window\(/ { found = 1; exit }
  END { exit (guarded && found) ? 0 : 1 }
' "$source"; then
  echo "gate: build_window( must stay behind the single-instance guard" >&2
  exit 1
fi

: "${DBUS_SESSION_BUS_ADDRESS:?run inside the user session}"

"$1" --reset &
first=$!
sleep 2
"$1" &
second=$!
sleep 2
test "$(pgrep -x ojuny-welcome | wc -l)" -eq 1
if command -v wmctrl >/dev/null 2>&1; then
  test "$(wmctrl -l -x | grep -c 'org.ojuny.Welcome')" -eq 1
fi
kill "$first" "$second" 2>/dev/null || true
