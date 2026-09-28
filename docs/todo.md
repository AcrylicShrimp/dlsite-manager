# Next steps

- Stable 3.4.0 is published. The packaged Linux update check passed with the
  explicit `Accept: */*` header; older clients with the failure need a manual
  download. Actual download/install/relaunch of a newer update remains unverified.

- Local OrbStack Linux GUI baseline is available; see `docs/linux-qa.md`.
  Published v3.3.0 startup/navigation and actual log-folder opening passed on
  Ubuntu 22.04 + IceWM/Thunar. Issue #47 remains on hold for the upstream Tauri
  fix per the maintainer's decision; do not restart reporter data collection.

- Packaged 3.4.0 Linux checks passed for sanitized environment metadata, native
  save/cancel/retry, complete diagnostic ZIP and actual log-folder opening. Support
  summary automatic copy fell back to a text field in this session. Verify native
  clipboard support, startup-failure export, browser links and other desktop systems.
- Production UI cutover is implemented from the approved checkpoint: shared Tailwind
  controls and semantic colors, one main scroller, compact library/full detail,
  account edit/MFA, download queue, Activity tabs and Settings tabs. See
  `docs/ui-redesign-preview.md`. Published 3.4.0 remains unchanged.
- App/Production workspace now mounts the real route/controllers against isolated
  IPC fixtures; Redesign/Compare is current cutover versus the approved prototype,
  not a frozen old-layout comparison. Shared tokens live in
  `src/lib/styles/workspace.css`; Tailwind is enabled in production too.
- Before a release, smoke-test the packaged app on native platforms, especially
  cover Save image (download, save/cancel/write errors), MFA above other dialogs,
  and webview focus/scroll rendering. Browser coverage uses mocked native IPC.
