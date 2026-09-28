# Production UI cutover review — 2026-09-28

Target: working-tree cutover after 9af167b. User-approved redesign and
preservation contract are recorded in docs/ui-redesign-preview.md. No release
or push is part of this task. Reviewers are fresh-context, read-only agents.

## Round 1 — BLOCK

Verified findings:

- Activity preferred the backend job title, hiding the resolved account name.
  Native account-sync titles contain an account ID. Fixed history and detail to
  use their supplied title projection; ProductionAppStory now uses native-shaped
  sync titles rather than masking the regression with friendly fixture text.
- Restored the accepted filter Show results shortcut and detail Back to top.
  Shared BackToTop handles main/detail scroll and reduced motion; it focuses the
  corresponding page/dialog heading. The main action previously focused its
  container instead of its heading.

An additional local check found that the production filter and TextInput focus
styles lacked the prototype's forced-colors fallback. Added a thin inset system
outline for that mode while retaining the normal contained focus treatment.

The reviewer reran both browser integration drivers successfully in Chromium and
WebKit. No reviewer edits. All findings were verified; none rejected.

Validation after fixes: Svelte check, targeted Chromium/WebKit browser checks for
native sync identity, main/detail heading focus, reduced-motion scroll and Show
results; Chromium forced-colors fallback. Native save dialogs remain a packaged
smoke-test item; browser IPC mocks cannot validate them.

## Round 2 — PASS

No blocking or actionable minor findings. The fresh reviewer independently ran
all 13 frontend tests, the Chromium/WebKit navigation-fix driver and the MFA/
diagnostics edge driver; all passed. They reviewed the route/controller/native
boundaries and confirmed all round-one corrections. No reviewer edits.

## Final status follow-up — PASS

Local verification after round two found that a persisted `downloading` product
without an individual work job lost its icon, unlike the approved prototype.
This can occur with a bulk parent job. Added the same stored-state fallback while
keeping actual active jobs authoritative and requiring a local path for the
available-folder icon. The browser case failed before the fix and passed after.

A third fresh reviewer checked only this final state projection and route inputs.
They independently passed the seven-case card check in Chromium/WebKit, plus active
jobs overriding downloaded state, cancellation, failed/cancelled stored states,
empty paths, downloading with an old path and distinct transfer/unpacking icons.
No findings and no reviewer edits.

Final result: PASS. Two full cutover review rounds and one bounded status follow-up.
No rejected or unresolved findings. Final Svelte check has zero diagnostics;
application and Storybook builds pass; 13 frontend tests, cargo check for the Tauri
adapter, cover validation test and 13 audit tests pass. Native-platform smoke
coverage remains the documented release gate, not a claim made by browser tests.
