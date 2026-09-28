# Workspace redesign preview — 2026-09-28

Status: visual direction accepted by the maintainer on 2026-09-28. Checkpoint
before production cutover; this Storybook implementation still uses fixtures.

The maintainer authorized a Storybook prototype and one-to-one comparison after
the reference discussion. Heroic informs layout only, not visual styling. The
download overview must represent concurrent work rather than one selected job.

## Open and compare

Run `pnpm storybook`, then open **Redesign / Compare / Side By Side**:

`http://localhost:6006/iframe.html?id=redesign-compare--side-by-side&viewMode=story`

- Compare all five views: Library, Downloads, Activity, Accounts, Settings.
- Both iframe canvases have exactly the same viewport (800×600, 1200×800 or
  390×700) and use the same synthetic fixture factory. Any fit-to-container scaling
  is applied equally *after* layout, preserving genuine responsive breakpoints.
- Use side-by-side mode or toggle A/B in the same position. Original-size mode
  preserves 100% scale with horizontal panning if the host is too narrow.
- Parent controls and navigation inside either canvas synchronize the selected
  view. Independent interactions remain local, so users can explore both versions.
- Switch between populated, long-list and empty scenarios. Long data contains
  72 works and 100 log events, including failed work. MFA scenarios show initial,
  rejected-code and submitting prompts in both canvases. Test code `123456`
  succeeds; other nonempty codes trigger a cleared retry prompt. No authentication
  request is sent. The submitting scenario stays busy for visual inspection.
- **Current** composes actual working-tree production views and AppShell. This
  includes the earlier, uncommitted Activity overflow patch, not the released
  3.4.0 layout. **Draft** is isolated under `src/stories/redesign/`.

## Draft choices to evaluate

- Quiet charcoal/sage styling, compact consistent page headings and actions,
  fewer enclosing cards and borders; primary lists use the content workspace.
- One vertical content scroller per screen. Related rows grow naturally. Detail
  dialogs temporarily own their own scroll and use native dialog focus handling.
- Sidebar overview shows aggregate running/queued counts, individual progress for
  both concurrently running fixture downloads and a link to failed work. Narrow
  mobile navigation omits the expanded overview; Downloads still has all counts.
- Downloads is for active queue management. Activity separates work history
  (terminal jobs plus account sync) and application logs. Export is a dialog, with
  a direct path from failed work to scoped diagnostics. This division is provisional.
- Library uses cover-first browsing and on-demand detail; Accounts edits in a
  dialog; Settings separates Storage and About & updates. Scrolled library
  position is preserved across detail dismissal and navigation.

## Authentication flow preservation

MFA is a job-triggered verification prompt, not an account setup toggle. The
comparison's MFA scenarios render the existing `TwoFactorDialog` in Current and
`DraftTwoFactorDialog` in Draft using the same `TwoFactorRequest` shape. Dedicated
Redesign/Workspace/TwoFactorRequired, TwoFactorRejected and TwoFactorSubmitting
stories make these transient states discoverable without a real account.

The draft uses the shared buttons, semantic colors and contained focus styling.
It identifies the account, focuses a single code input with one-time-code
completion, clears codes on new requests, and exposes rejected/retry and busy
states. Submission trims the value and blocks empty/duplicate submissions; the
preview disables dismissal while verifying. Outside that state, Cancel, close
and Escape all cancel. Story timers are disposed on scenario changes and no
entered code is stored in logs or persistent state. Success/cancel feedback and
Try again belong to the fixture harness. Backend events, request queueing and
native authentication remain the existing production controller's responsibility.

## Library feedback and preservation contract

The maintainer accepted the thumbnail/title/maker grid, but rejected the loss of
filter and detail capabilities. Keep the grid concise while preserving existing
information and actions behind it. A visual redesign is not authorization to
remove functionality, even from the interactive acceptance prototype.

| Existing capability | Draft placement |
| --- | --- |
| Latest purchase / published / title sorting | Full Filters disclosure, using the production `LibraryFilters` component |
| Account, owned/local-only, age, type and maker selection | Same full filter disclosure; combinations affect fixture results |
| Custom tag include / exclude / clear | Existing three-state tag chips, counts and reset; include matches any selected tag, exclude rejects any selected tag |
| Search title, maker, work ID, credits, source and custom tags | Main Library search |
| Title, work ID, maker name/ID, type, age, size, detail sync | Compact detail header; maker ID and sync in Work information disclosure |
| Translated title/maker variants | Work information disclosure |
| Maker, CV, illustration, scenario, creator, music, other credits | Credits section with copy controls and explicit missing values |
| Registered, published, updated and purchase dates | Collapsed Dates section, including first and latest purchase |
| Multiple owning accounts and purchase dates / local-only state | Header account summary; Ownership & purchases disclosure |
| Download status, unpack policy, local path, errors | Status/action and errors near the top; Download details disclosure retains paths, policy, bytes and timestamps |
| Custom tag add/remove/copy | Custom Tags directly below the header, beside DLsite tags; edits persist in the preview and update filtering/search |
| Cover preview, copy fields, DLsite link | Cover opens a separate native image dialog with Save image and close; saving/external effects remain preview callbacks |
| Archive-only, mark downloaded, re-download, delete | Visible action group below the identity header, beside Download/Open folder; simulated native effects |

Follow-up visual feedback: the first detail screen prioritizes cover, identity,
kind/age, ownership summary, status/action and editable tags. Credits stay in a
compact two-column grid; metadata groups start collapsed. Failures stay visible
above the tags, never solely inside a disclosure. No fields or actions were removed.
`DraftWorkBadges` shares kind/age rendering between cards and details: existing
category hues (audio yellow, image/comic green, video pink, game purple, voice
comic cyan), plus labeled age chips. Downloaded markers sit in the opposite
corner so they do not cover the chips. The former completed checkmark is now a
status icon: no mark for not downloaded, clock for queued, download arrow for
transferring, open box for unpacking, folder for available locally. Tooltips and
accessible cover names expose the status in text. Active jobs take precedence
over saved state; cancelling a preview job clears its stale downloading state.
One of the two active fixtures now unpacks so all requested states can be compared. Main Back to top is a 40px icon-only circle
with an accessible name and tooltip. Text actions and sidebar summary buttons
have horizontal hover padding; title links keep their text-only hover treatment.

Current and Draft now both expose working fixture filters and product details.
They use the same query/detail fixtures; Current still renders the production
filter/detail components. Shared fixture queries are Storybook-only projections,
not a replacement for the Rust query implementation.

The main content area displays a Back to top button after 300px of scrolling;
returning to the top focuses the page heading and honors reduced-motion settings.
Product details also include a return-to-top action. Detail dismissal preserves
the library position. Filter disclosure has a Show results action so large filter
sets do not require manually finding the start of the grid.
The detail close control stays visible while scrolling. Maker and custom-tag
facet counts follow other active filters while excluding their own selection,
matching the production facet boundary.

## Control layout rules

Draft controls use Tailwind utilities inside reusable components:
`DraftButton` (primary/secondary/text/icon/navigation), `DraftChoiceGroup`
(tabs and state filters), `DraftListRow` (padded clickable history/log rows),
`DraftActivityRows`, `DraftDownloadRow`, `DraftAccountRow`, `DraftSidebar`,
`DraftProgress`, `DraftTagChip`, `DraftWorkBadges`, `DraftAppMark`, `DraftSearchField` and
`DraftDisclosure`. These are story-only primitives while the
redesign is under review; the production Button and baseline remain unchanged.
Page styles position components; the shared primitive owns padding, borders,
hover/focus and disabled states. History/log rows share 16px insets; download
and account rows follow the same spacing. All ordinary draft actions, including
form/detail actions, use the button primitive. Page-specific detail/grid layout
still has scoped CSS; production `LibraryFilters` is reused through a theme adapter.
`Redesign / Controls / Shared` isolates hover, focus, disabled, selection, long
row labels, custom tags and kind/age badges for visual inspection.

Keep controls within their parent's content width. Do not compensate for parent
padding by widening children beyond 100% or shifting them with negative margins.
Navigation and queue rows share the same outer width and 10px horizontal inset.
The tag copy area's own padding belongs to its clickable/hover area, not to an
extra surrounding offset. Dialogs have zero outer padding; their sticky header
and body each own their padding, with the header at `top: 0`.
Comparison iframe scaling is intentional equal-viewport presentation, not a
workaround for product layout. Cover zoom is an image interaction, not alignment.

Library and log searches share `DraftSearchField`: the inner input has no outline;
focus changes only the existing outer border, without changing bounds. The clear
button restores input focus after clearing the library query.

Detail actions stay above the tags instead of inside metadata disclosures. The
sticky close button is the single dismissal control; no duplicate Back to library
button appears at the bottom. Metadata uses `DraftDisclosure`, with padded summary
rows and the shared keyboard-only row focus rather than a large outline.
Native details/summary behavior preserves Enter/Space toggling; each disclosure
arrow follows its own open state, including nested groups.

`DraftImagePreview` opens above work details without changing the underlying
layout or scroll. Its native modal contains the image, Save image and close;
Escape dismisses only the image and focus returns to the cover without scrolling.
Save reports a simulated save-dialog request inside the preview; native image
fetching and file persistence remain part of production integration.

Focus is defined once in `draft.css` through reusable `dm:draft-focus-*`
utilities, including standalone page controls and the production-filter theme
bridge. Do not add per-screen outside outlines. Buttons, tabs and chips get an
inset bottom marker; list/disclosure rows get a background and left marker;
text actions get an underline; image controls get a bottom overlay; input fields
change their existing border. Primary buttons use on-accent focus color for
contrast. These indicators do not change layout and use `:focus-visible` except
text entry fields, whose border also follows pointer focus. Forced-colors mode
retains a thin inset system-color outline where shadows would be suppressed.
`draft-focus`, `draft-focus-bg` and `draft-focus-control` (shadow) are shared tokens.

About groups the shared square app mark beside the app name and version, with
the update action alongside (wrapping when needed) and project/license links below.
The sidebar uses the same mark component at a smaller size.

## Semantic colors and Tailwind boundary

`src/stories/redesign/draft.css` owns the draft palette. Use semantic utilities or
CSS variables, not literal color values in components. Roles include:

| Role | Token family |
| --- | --- |
| Workspace, sidebar, panels, inputs | `draft-background`, `draft-sidebar`, `draft-surface`, `draft-input` |
| Main, secondary and placeholder text | `draft-ink`, `draft-dim`, `draft-placeholder` |
| Borders, focus, primary action | `draft-line*`, `draft-accent`, `draft-on-accent` |
| Hover and selection | `draft-hover`, `draft-row-hover`, `draft-selected*` |
| Failure, queue, active transfer, local availability | `draft-error*`, `draft-queued`, `draft-in-progress`, `draft-available` |
| Custom tags and catalog genres | `draft-tag-*`, `draft-genre-*` |
| Work kinds, age ratings and ownership source | `kind-*`, `age-*`, `source-*` |

Tailwind v4.3 is installed and its Vite plugin runs only in Storybook for now.
The draft stylesheet uses a `dm:` prefix and omits Preflight, preserving the
production comparison canvas and existing stories. The prefixed variables are
`--dm-color-…`; changing these roles updates both Tailwind components and remaining
scoped layout styles. Keep kind/age/source roles independent of the general accent.
The production filter adapter maps its existing variables/category colors to the
same roles without modifying the baseline component.

Integration follows Tailwind's [Vite setup](https://tailwindcss.com/docs/installation/using-vite),
[selective imports](https://tailwindcss.com/docs/preflight#disabling-preflight) and
[theme variables](https://tailwindcss.com/docs/theme). A production migration
should decide the final shared theme boundary before enabling this in app builds.

## UI copy

Omit slogans and explanations that simply restate a heading or obvious behavior.
The draft no longer has page/brand taglines, normal-state logging assurances,
export introduction, redundant queued descriptions, download destination/future
list explanations, or the account credential reassurance. Export keeps scope
selection and the export action. Keep actionable empty-state recovery, errors,
unsaved-state feedback, storage-folder distinctions and bulk-download scope.
Prototype-only feedback and the sample-credential notice remain to distinguish
simulated actions from real effects.

## Prototype boundary

Search, full filter groups, custom tags, tabs, dialogs, queue cancellation/enqueue, account editing
and settings save are in-memory interactions. No account, disk, updater or network
operation is performed. Native actions display preview feedback; exports do not
write a ZIP. Current-view callbacks report actions; diagnostic IPC is mocked only
inside its isolated iframe and cleaned up on unmount.

This is not complete production behavior: native/destructive account/work actions,
real authentication and every diagnostic recovery
state still need placement/design before production implementation. Do not treat
their omission here as approval to remove existing functionality. The mockups do
not change the app route, theme, native APIs, release version or production views.

## Validation

- `pnpm check`: no errors or warnings.
- `pnpm build-storybook`: passed (existing large-bundle advisory).
- Browser checks in Chromium and WebKit at all three viewport sizes: all five
  views without horizontal content overflow; wheel scrolling over library/log
  rows; no nested draft list scrollers; dialog dismissal and navigation preserve
  library scroll position; search/reset; failed work → export; account save;
  settings save. Comparison navigation, data and viewport synchronization, empty
  states and A/B visibility passed in both engines.
- axe WCAG 2 A/AA checks on the five default draft workspaces at 800×600 found no
  violations. This is a scoped automated check, not full accessibility approval.
- Temporary browser driver/captures: ignored `.linux-qa/redesign-check.cjs` and
  `.linux-qa/captures/redesign/`. No packaged Tauri runtime test for this story-only
  prototype. No production UI acceptance, commit or release is implied.
- Library follow-up: Chromium/WebKit checks at all three sizes cover combined
  filters, tag inclusion/exclusion/reset, full/sparse/failed details, tag editing
  reflected in facets/search, both baseline and draft detail access, and Back to
  top. Additional checks cover account+maker combinations, title sorting, OR
  semantics for included tags, persistent close control and reduced-motion/focus
  behavior. Scoped axe checks of expanded filters and detail found no WCAG 2 A/AA
  violations. Driver: `.linux-qa/redesign-library-check.cjs`.

- Visual refinement checks: Chromium/WebKit at all three sizes passed shared
  button padding, kind/age badges, visible/editable tags, collapsed metadata field
  access, visible errors, no horizontal dialog overflow and icon-only top behavior.
  Svelte check and Storybook build passed after the refinement.

- Tailwind/component pass: Svelte check, frontend tests, Storybook build and
  app build checked separately. Chromium/WebKit at 800×600, 1200×800 and 390×700
  passed five-view flows, actual scrolling, A/B synchronization, sidebar bounds,
  padded tag hit areas, sticky dialogs and actions. Shared-controls tests cover
  hover/focus/disabled/narrow layouts. Runtime token overrides update the draft
  across views while kind colors and baseline styling remain unchanged.

Next: cut over the accepted visuals to production components and existing native
controllers. Preserve the capability matrix and real loading/error/cancellation
flows; fixture actions, test MFA codes and hardcoded versions must stay in stories.
