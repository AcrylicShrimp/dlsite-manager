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
- The redesign checkpoint includes the earlier Activity overflow patch,
  but the maintainer rejected its cramped, nested-scroll UX. Redesign scrolling,
  navigation and list/detail flows consistently across the app before treating
  that patch as release-ready. Reference research: `docs/ui-reference-research.md`.
  The redesign direction is now accepted for cutover; the published 3.4.0 remains unchanged.
- Five-screen Storybook redesign draft and equal-viewport A/B comparison are now
  available under Redesign/Compare; see `docs/ui-redesign-preview.md`. The maintainer
  accepted this visual checkpoint on 2026-09-28 and authorized production cutover. Thumbnail/title/maker grid
  direction is accepted; full filtering and product detail preservation is required
  and restored in the prototype, along with Back to top. Production integration must preserve native action
  flows, loading/error states, query pagination and job cancellation.
  The draft now uses Tailwind-based shared controls and semantic colors in
  `src/stories/redesign/draft.css`; inspect Redesign/Controls alongside the A/B
  workbench. Tailwind is currently enabled for Storybook only; production setup is part of cutover.

- MFA is now represented in the redesign comparison (initial/rejected/submitting
  scenarios and interactive retry/cancel flow). During production integration,
  connect the accepted dialog to the existing TwoFactorController/native events;
  do not carry over the Storybook test-code simulation.
