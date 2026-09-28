#!/usr/bin/env bash
set -euo pipefail

# Run as root inside the dedicated Ubuntu 22.04 OrbStack machine.
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends \
  ca-certificates curl file libfuse2 libgtk-3-0 libwebkit2gtk-4.1-0 libglib2.0-bin \
  libayatana-appindicator3-1 librsvg2-2 libsecret-1-0 gnome-keyring \
  dbus-x11 xvfb icewm x11vnc novnc websockify imagemagick \
  xdotool x11-utils wmctrl thunar xdg-utils netsurf-gtk \
  fonts-dejavu-core fonts-noto-cjk

install -d -o qa -g qa /home/qa/.local /home/qa/.local/lib /home/qa/.local/state \
  /home/qa/.local/share /home/qa/.config /home/qa/.local/lib/dlsite-qa /home/qa/.local/state/dlsite-qa \
  /home/qa/.config/icewm /home/qa/Applications /home/qa/qa-captures /home/qa/qa-folder
cat > /home/qa/.config/icewm/preferences <<'EOF'
ShowTaskBar=0
TaskBarAutoHide=1
EOF
chown qa:qa /home/qa/.config/icewm/preferences

cat > /etc/systemd/system/dlsite-qa-desktop.service <<'EOF'
[Unit]
Description=dlsite-manager local Linux QA desktop
After=network.target

[Service]
Type=simple
User=qa
WorkingDirectory=/home/qa
RuntimeDirectory=dlsite-qa
RuntimeDirectoryMode=0700
Environment=XDG_RUNTIME_DIR=/run/dlsite-qa
ExecStart=/usr/bin/dbus-run-session -- /home/qa/.local/lib/dlsite-qa/session.sh
KillMode=control-group
TimeoutStopSec=10
EOF

cat > /etc/systemd/system/dlsite-qa-app.service <<'EOF'
[Unit]
Description=dlsite-manager AppImage under test
Requires=dlsite-qa-desktop.service
After=dlsite-qa-desktop.service
PartOf=dlsite-qa-desktop.service

[Service]
Type=simple
User=qa
WorkingDirectory=/home/qa
ExecStart=/home/qa/.local/lib/dlsite-qa/guest.sh launch
KillMode=control-group
TimeoutStopSec=10
EOF
systemctl daemon-reload
