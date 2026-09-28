# Storybook cutover readiness review

Scope: accepted visual prototype at checkpoint `79683c9`, not production cutover
implementation or a new aesthetic review. Preserve existing capabilities and
identify assumptions that must not migrate into the application. Native actions,
controller wiring and fixture data are intentionally simulated in Storybook.

## Round 1

- Reviewer: `/root/storybook_cutover_review_1`, fresh context, read-only.
- Baseline: HEAD `79683c915b69d80f7c4aa47af593e1b522ea77aa`; clean status/diff.
  Status remained clean after the reviewer returned.
- Verdict: **BLOCK**.
- Verified blocking finding: account Login had become required/email-only,
  blocking edits to the existing credentialless account and non-email IDs.
  Confirmed against production AccountEditor/route save semantics. Removed the
  restriction, kept nullable trimmed login values, and also preserved optional
  passwords for newly created sources.
- Verified minor finding fixed: active downloads/unpacking were passed as a
  boolean `queued` to work details. Pass the job snapshot and derive its current
  state/phase while retaining busy guards.
- Verified worthwhile minor fixes: cancel controls now honor cancellable and
  cancelling states; account enabled status no longer claims “Connected.”
- Verified integration notes retained: account sync/cancel/status/removal paths,
  real job classification and nullable/unit-aware progress still need production
  connection. Concrete destinations/acceptance gates are recorded in
  [the preview contract](../ui-redesign-preview.md#cutover-requirements-beyond-the-visual-fixture).
  A complete native mock framework is outside this review's scope.
- No reviewer claims rejected or left inconclusive.
- Local validation: Svelte check, zero diagnostics. Chromium/WebKit at 800/390px
  reproduced and then passed credentialless account edits, text login IDs,
  credentialless creation, phase labels and action guards. Direct row mounts
  confirmed non-cancellable/cancelling jobs cannot invoke Cancel.

## Round 2

- Reviewer: `/root/storybook_cutover_review_2`, fresh context, read-only.
- Target: the checkpoint plus verified changes in DraftWorkspace,
  DraftProductDetail, DraftDownloadRow, DraftAccountRow and the preservation doc.
  No reviewer edits; status/diff remained unchanged during review.
- Verdict: **PASS_WITH_NOTES**. No blocking findings.
- Independently reran the Chromium/WebKit regression driver at both widths and
  checked the diff. Confirmed account semantics, phase labels, busy/cancel guards
  and mock/production authentication separation.
- Verified retained notes: account operations and real job classification,
  progress and status mapping are cutover gates, not a reason to extend the mock
  framework. Activity's simplified status fallback must not replace production
  labels. Also verified production `productDownloadActionDisabled` disables all
  active work actions and downloaded records without a local path; explicitly
  added these real-data cases to the primary-action integration gate. The fixture
  does not yet cover downloaded-with-missing-path or simultaneous active jobs.
- No claims rejected or inconclusive. These remaining notes are intentionally
  deferred to production integration; this verdict does not certify native flows.
- Final local validation: Svelte check and all 13 frontend tests passed; targeted
  browser checks passed in both rounds. Storybook build recorded in work log.

## Outcome

Two rounds: BLOCK → verified fixes → PASS_WITH_NOTES. The visual checkpoint can
serve as the cutover reference with the explicit integration gates above. Review
fixes remain separate from checkpoint commit 79683c9; no cutover or release was
performed as part of this review.
