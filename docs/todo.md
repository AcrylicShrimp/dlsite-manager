# Next steps

- Stable 3.4.0 is published. The packaged Linux update check passed with the
  explicit `Accept: */*` header; older clients with the failure need a manual
  download. Actual download/install/relaunch of a newer update remains unverified.

- Local OrbStack Linux GUI baseline is available; see `docs/linux-qa.md`.
  Published v3.3.0 startup/navigation and actual log-folder opening passed on
  Ubuntu 22.04 + IceWM/Thunar. Match issue #47's reporter environment and test
  downloaded-work opening before drawing conclusions about that issue.

- Packaged 3.4.0 Linux checks passed for sanitized environment metadata, native
  save/cancel/retry, complete diagnostic ZIP and actual log-folder opening. Support
  summary automatic copy fell back to a text field in this session. Verify native
  clipboard support, startup-failure export, browser links and other desktop systems.
- Improve Activity layout at 800×600: after export status or support-summary text
  expands, lower panels can fall outside the viewport. Current narrow-window smoke
  checks accessed the log-folder button before expanding those sections.
