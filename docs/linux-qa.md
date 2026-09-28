# Local Linux GUI checks

`scripts/linux-qa.sh` provisions a dedicated OrbStack machine named `dlsite-qa`.
It runs the actual Linux AppImage, with an X11 desktop accessible through noVNC.
Playwright can click this desktop through a browser; it does not inspect the
Tauri webview DOM. No app source or production opener behavior is changed.

## Setup and use

Requires running OrbStack (`orbctl`), `gh` for release downloads, and Node/npm
only for optional browser automation. Run from the repository:

```sh
scripts/linux-qa.sh setup
scripts/linux-qa.sh release v3.3.0
scripts/linux-qa.sh browser-setup
scripts/linux-qa.sh browser screen
```

Open <http://127.0.0.1:6080/vnc.html?autoconnect=true&resize=scale> and enter the
password in `.linux-qa/vnc-password`. A browser panel may present a proxied URL.
The password, downloaded assets, optional Playwright installation, and captures
stay in the ignored `.linux-qa/` directory. Guest app data persists between runs;
it is separate from the Mac app's profile.

```sh
scripts/linux-qa.sh app /absolute/path/to/custom.AppImage
scripts/linux-qa.sh view
scripts/linux-qa.sh browser click 88 563
scripts/linux-qa.sh browser type 'test query'
scripts/linux-qa.sh browser key Enter
scripts/linux-qa.sh capture folder-opening
scripts/linux-qa.sh exec wmctrl -l
scripts/linux-qa.sh status
scripts/linux-qa.sh stop
```

Browser coordinates are relative to the 800×600 Linux desktop, including the
window title bar. Each browser command saves a PNG. `capture` saves desktop/app
PNGs, window inventory, app SHA-256, OS information, selected desktop environment
variables, and the two services' journal output. These are raw local QA artifacts,
not the application's sanitized support export. App-owned log files remain in
the guest profile and can be opened from Activity.

`stop` stops the app and desktop services; it retains the machine and data.
`release` verifies the published checksum, copies the artifact into the guest,
and starts it. `view` starts only the desktop if it was stopped; use `release`
or `app` to start the application. Re-running `setup` updates guest scripts and
packages; stop/start the desktop to apply session changes.

## Environment and boundaries

- Ubuntu 22.04 amd64, 4 CPUs, 4 GiB RAM, 20 GiB disk limit; on Apple Silicon this
  exercises the x86 release through OrbStack's translation support.
- Isolated machine: no Mac home mount or SSH-agent forwarding. Artifacts are
  explicitly copied in. Internet access remains available.
- Xvfb + IceWM, software rendering (`LIBGL_ALWAYS_SOFTWARE=1`), Thunar and NetSurf.
  MIME handlers and a private D-Bus session are configured for desktop integration.
- Authenticated VNC and noVNC listen only on guest loopback. OrbStack makes the
  noVNC listener available on Mac localhost; no SSH tunnel or global OrbStack
  networking changes are needed. See [OrbStack networking](https://docs.orbstack.dev/machines/network).
- This is a baseline for startup, layout, native dialogs and external opening.
  It does not reproduce GNOME/KDE, Wayland, GPU drivers, or every distribution.
  For issue-specific failures, match the reporter's environment separately.
- Published v3.3.0 predates the logging overhaul on main. Testing this release
  does not validate the new diagnostic export; supply a newly built AppImage for it.

## Recorded checks — 2026-09-28

Published v3.3.0 amd64 AppImage SHA-256:
`2cbefe1109e4e243a8c49e835d850fcf9c1ca4260a7750f9ba763d623647c718`.

- AppImage mounted and started normally; extraction fallback was not used.
- Browser-driven navigation rendered Library, Downloads, Settings and Activity.
- The white bottom rectangle from the AppImage catalog screenshot was absent in
  local Library captures. This is not a root-cause determination: startup ordering,
  window manager settings and system libraries differ from the catalog runner.
- `xdg-open /home/qa/qa-folder` displayed Thunar with the expected directory.
- After quitting Thunar, clicking Activity → Open Folder displayed Thunar with
  `/home/qa/.local/share/dev.ashrimp.dlsite-manager/logs/` and its log files.
  Evidence: `.linux-qa/captures/20260928T030805Z-app-open-DhBa/`.
- Setup rerun and desktop/app stop/start succeeded. Shell syntax and Node syntax
  checks passed. No account login, real download, downloaded-work opening, native
  save/cancel/export, browser-link launch, GPU or Wayland acceptance was performed.

Issue #47 remains unverified in the reporter's environment. Its comments do not
specify the distribution, desktop/session or file manager. A successful local log
folder opening is narrower evidence than resolving the reported work-folder failure.
