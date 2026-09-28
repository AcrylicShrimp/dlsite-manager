#!/usr/bin/env bash
set -euo pipefail
repo=$(cd "$(dirname "$0")/.." && pwd)
machine=dlsite-qa
state="$repo/.linux-qa"
guest=/home/qa/.local/lib/dlsite-qa/guest.sh
mkdir -p "$state"
chmod 700 "$state"
run() { orbctl run -m "$machine" -w /home/qa "$@"; }
command=${1:-help}
shift || true
case "$command" in
  setup)
    if ! orbctl list -q | grep -qx "$machine"; then
      orbctl create --arch amd64 --isolated --user qa --memory 4G --cpus 4 --disk 20G ubuntu:22.04 "$machine"
    fi
    # Refuse a same-named machine with a different distro, arch or account.
    run bash -c 'source /etc/os-release; [[ "$ID:$VERSION_ID:$(dpkg --print-architecture):$(id -un)" == "ubuntu:22.04:amd64:qa" ]]'
    orbctl run -m "$machine" -u root -w /root bash -s < "$repo/scripts/linux-qa/provision.sh"
    COPYFILE_DISABLE=1 tar -C "$repo/scripts/linux-qa" -cf - session.sh guest.sh |
      run tar -xf - -C /home/qa/.local/lib/dlsite-qa
    run chmod +x /home/qa/.local/lib/dlsite-qa/session.sh "$guest"
    if [[ ! -f "$state/vnc-password" ]]; then
      openssl rand -hex 4 > "$state/vnc-password"
      chmod 600 "$state/vnc-password"
    fi
    run bash -c 'read -r password; x11vnc -storepasswd "$password" /home/qa/.local/state/dlsite-qa/vnc.pass >/dev/null 2>&1; chmod 600 /home/qa/.local/state/dlsite-qa/vnc.pass' < "$state/vnc-password"
    echo 'Setup complete. Use: scripts/linux-qa.sh release'
    ;;
  release)
    tag=${1:-v3.3.0}
    [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-rc\.[0-9]+)?$ ]] || exit 2
    version=${tag#v}
    name="dlsite-manager_${version}_amd64.AppImage"
    mkdir -p "$state/artifacts/$tag"
    if [[ ! -f "$state/artifacts/$tag/$name" ]]; then
      gh release download "$tag" --repo AcrylicShrimp/dlsite-manager \
        --pattern "$name" --pattern dlsite-manager-linux-SHA256SUMS.txt --dir "$state/artifacts/$tag"
    fi
    # Verify the published checksum before copying or running the binary.
    (cd "$state/artifacts/$tag"; awk -v name="$name" '$2 == name { print }' dlsite-manager-linux-SHA256SUMS.txt | shasum -a 256 -c -)
    "$0" app "$state/artifacts/$tag/$name"
    ;;
  app)
    [[ -f "$1" ]] || { echo 'AppImage not found' >&2; exit 1; }
    run sudo systemctl stop dlsite-qa-app
    # Explicit copy also works with isolated machines; no Mac home mount is needed.
    run bash -c 'cat > /home/qa/Applications/test.AppImage; chmod +x /home/qa/Applications/test.AppImage' < "$1"
    run "$guest" app /home/qa/Applications/test.AppImage
    "$0" view
    ;;
  view)
    run sudo systemctl start dlsite-qa-desktop
    # OrbStack forwards guest loopback listeners directly to Mac localhost.
    curl --fail --silent --show-error --retry 10 --retry-connrefused --retry-delay 1 \
      --max-time 3 http://127.0.0.1:6080/vnc.html >/dev/null
    echo 'Desktop: http://127.0.0.1:6080/vnc.html?autoconnect=true&resize=scale'
    echo "VNC password: $state/vnc-password"
    ;;
  capture)
    path=$(run "$guest" capture "${1:-screen}")
    name=${path##*/}
    mkdir -p "$state/captures/$name"
    run tar -C "$path" -cf - . | tar -C "$state/captures/$name" -xf -
    echo "$state/captures/$name"
    ;;
  exec) run "$guest" exec "$@" ;;
  browser-setup)
    npm install --prefix "$state/browser" --no-audit --no-fund playwright@1.63.0
    "$state/browser/node_modules/.bin/playwright" install chromium
    ;;
  browser) node "$repo/scripts/linux-qa/browser.cjs" "$@" ;;
  status) run "$guest" status ;;
  stop)
    run sudo systemctl stop dlsite-qa-app dlsite-qa-desktop
    ;;
  *) echo 'Usage: scripts/linux-qa.sh setup | release [v3.3.0] | app FILE | view | capture [LABEL] | exec COMMAND... | browser-setup | browser [screen|click X Y|key KEY|type TEXT] | status | stop' ;;
esac
