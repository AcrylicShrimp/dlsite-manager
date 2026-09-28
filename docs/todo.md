# Next steps

- Fix updater request compatibility with GitHub release assets: on 2026-09-28,
  `Accept: application/json` returned HTTP 500 on Mac and Linux, while `Accept: */*`
  returned valid v3.3.0 metadata. Tauri updater 2.11.0 sends the former by default.
  Verify an explicit header override through the packaged app before release.

- Local OrbStack Linux GUI baseline is available; see `docs/linux-qa.md`.
  Published v3.3.0 startup/navigation and actual log-folder opening passed on
  Ubuntu 22.04 + IceWM/Thunar. Match issue #47's reporter environment and test
  downloaded-work opening before drawing conclusions about that issue.

- Validate the diagnostic build as a packaged Linux AppImage: inherited environment,
  original opener dispatch, and ZIP export. Confirm actual folder/browser opening
  separately; issue #47 remains unverified. Child configuration/output is unobserved.
- Verify native save/cancel/copy interactions and startup-failure export on supported
  desktop platforms. Automated core/frontend tests and macOS packaging are recorded
  in `docs/work-log.md`.
- Prepare the next stable release after implementation review and platform checks;
  version bump/publication are separate from this implementation turn.
