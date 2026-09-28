# Next steps

- Updater checks now explicitly send `Accept: */*` to avoid GitHub's HTTP 500
  response to the default JSON Accept header. Verify the packaged 3.4.0 build
  before publishing; older clients may require a manual download.

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
- Publish stable 3.4.0 after release builds and packaged Linux checks; the maintainer
  explicitly authorized main push and stable publication on 2026-09-28.
