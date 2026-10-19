# Next steps

- Issue #49 is implemented through three completed plans, in dependency order:
  [localized JSON as the name source](plans/2026-10-19.00-use-localized-names-as-storage-source.md),
  [persist and apply language priority](plans/2026-10-19.01-apply-name-language-priority.md),
  then [edit priority in Settings](plans/2026-10-19.02-edit-name-language-priority-in-settings.md).
  All three steps are complete. Database upgrade, Rust/frontend checks, builds, and mocked-IPC browser regression checks passed.

- Preparing 3.6.0 for the name-language preference feature; see
  [release notes](releases/3.6.0.md). Version metadata is synchronized and local
  workspace/frontend release checks passed. Platform artifacts will be built
  into a draft release; native desktop acceptance remains pending.

- Stable 3.5.0 is published with the redesigned workspace and native-QA fixes.
  All platform builds and uploaded artifact signatures/checksums passed; public
  latest.json serves 3.5.0. Linux startup, update checks and correlated diagnostic
  export passed. Actual installation through the updater remains unverified.

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
  account edit/MFA, download queue, Activity tabs and Settings tabs. Page headers,
  query/action toolbars and edit footers now share placement and spacing rules. See
  `docs/ui-redesign-preview.md`. These changes shipped in 3.5.0.
- App/Production workspace now mounts the real route/controllers against isolated
  IPC fixtures; Redesign/Compare is current cutover versus the approved prototype,
  not a frozen old-layout comparison. Shared tokens live in
  `src/lib/styles/workspace.css`; Tailwind is enabled in production too.
- Remaining platform checks include
  cover Save image (download, save/cancel/write errors), MFA above other dialogs,
  and native Windows UI. macOS maintainer QA and the 3.5.0 Linux smoke check passed;
  broader browser coverage uses mocked native IPC.

- Refresh README screenshots to show the redesigned interface; explicitly deferred
  by the maintainer until after the next release.
