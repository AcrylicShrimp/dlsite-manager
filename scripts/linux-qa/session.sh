#!/usr/bin/env bash
set -euo pipefail
umask 077
export DISPLAY=:99 XDG_CURRENT_DESKTOP=ICEWM XDG_SESSION_TYPE=x11
# Software rendering is deliberate for this headless baseline, not GPU acceptance.
export LIBGL_ALWAYS_SOFTWARE=1
Xvfb "$DISPLAY" -screen 0 800x600x24 -nolisten tcp &
for ((i=0; i<100; i++)); do
  if xdpyinfo >/dev/null 2>&1; then break; fi
  sleep 0.1
done
xdpyinfo >/dev/null
icewm &
gnome-keyring-daemon --start --components=secrets
xdg-mime default thunar.desktop inode/directory
xdg-mime default netsurf-gtk.desktop x-scheme-handler/http x-scheme-handler/https

printf 'export DISPLAY=%q XDG_RUNTIME_DIR=%q DBUS_SESSION_BUS_ADDRESS=%q\n' \
  "$DISPLAY" "$XDG_RUNTIME_DIR" "$DBUS_SESSION_BUS_ADDRESS" > "$XDG_RUNTIME_DIR/session.env"
printf 'export XDG_CURRENT_DESKTOP=ICEWM XDG_SESSION_TYPE=x11 LIBGL_ALWAYS_SOFTWARE=1\n' >> "$XDG_RUNTIME_DIR/session.env"

# VNC is authenticated and both listeners are bound to the guest loopback only.
x11vnc -display "$DISPLAY" -localhost -rfbport 5900 -forever -shared \
  -rfbauth /home/qa/.local/state/dlsite-qa/vnc.pass -noxdamage &
websockify --web=/usr/share/novnc 127.0.0.1:6080 127.0.0.1:5900 &
wait -n
