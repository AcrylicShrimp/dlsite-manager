#!/usr/bin/env bash
set -euo pipefail
state=/home/qa/.local/state/dlsite-qa
command=${1:-status}
shift || true
session() {
  for ((i=0; i<100; i++)); do
    if [[ -f /run/dlsite-qa/session.env ]]; then
      source /run/dlsite-qa/session.env
      return
    fi
    sleep 0.1
  done
  echo 'Desktop is not ready; inspect sudo journalctl -u dlsite-qa-desktop' >&2
  exit 1
}
case "$command" in
  launch)
    session
    IFS= read -r app < "$state/app-path"
    exec "$app"
    ;;
  app)
    app=$(realpath -- "$1")
    [[ -f "$app" && -x "$app" ]] || { echo 'Expected executable AppImage' >&2; exit 1; }
    printf '%s\n' "$app" > "$state/app-path"
    sudo systemctl start dlsite-qa-desktop
    session
    sudo systemctl restart dlsite-qa-app
    for ((i=0; i<100; i++)); do
      if ! systemctl is-active --quiet dlsite-qa-app; then
        echo 'App exited; inspect scripts/linux-qa.sh status or capture' >&2
        exit 1
      fi
      if xdotool search --onlyvisible --class '^dlsite-manager$' >/dev/null; then exit 0; fi
      sleep 0.1
    done
    echo 'App did not create a visible window within 10 seconds' >&2
    exit 1
    ;;
  capture)
    session
    label=${1:-screen}
    [[ "$label" =~ ^[a-zA-Z0-9_-]+$ ]] || { echo 'Use a simple capture label' >&2; exit 1; }
    output=$(mktemp -d "/home/qa/qa-captures/$(date -u +%Y%m%dT%H%M%SZ)-${label}-XXXX")
    import -window root "$output/desktop.png"
    window=$(xdotool search --onlyvisible --class '^dlsite-manager$' | head -n 1 || true)
    if [[ -n "$window" ]]; then import -window "$window" "$output/app.png"; fi
    xwininfo -root -tree > "$output/windows.txt"
    { cat /etc/os-release; uname -a; dpkg --print-architecture; } > "$output/platform.txt"
    env | sort | grep -E '^(DISPLAY|XDG_CURRENT_DESKTOP|XDG_SESSION_TYPE|LIBGL_ALWAYS_SOFTWARE|APPIMAGE|APPDIR|LD_LIBRARY_PATH|LD_PRELOAD)=' > "$output/session.txt"
    if [[ -f "$state/app-path" ]]; then
      IFS= read -r app < "$state/app-path"
      sha256sum -- "$app" > "$output/app-sha256.txt"
    fi
    sudo journalctl -b -u dlsite-qa-app -u dlsite-qa-desktop -n 2000 --no-pager > "$output/journal.txt"
    echo "$output"
    ;;
  exec)
    session
    exec "$@"
    ;;
  status)
    systemctl --no-pager status dlsite-qa-desktop dlsite-qa-app || true
    ;;
  *) echo 'Commands: app PATH | capture LABEL | exec COMMAND... | status' >&2; exit 2 ;;
esac
