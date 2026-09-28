# UI reference research — 2026-09-28

Status: discussion material, not an approved design or implementation plan.

The maintainer requested an application-wide UX redesign after rejecting the
Activity layout that stacks three panels with nested scrolling. Main content
should scroll naturally across the available page area. Consistent navigation,
list/detail behavior and secondary actions must apply across all views.

## Product observations

These observations come from official documentation and published screenshots,
not hands-on testing of the products. Screenshots can depict older releases;
they establish layout examples, not current-version or wheel-behavior guarantees.

| Product | Observed pattern | Relevance and limits |
| --- | --- | --- |
| [Heroic Games Launcher](https://heroicgameslauncher.com/) | Sidebar navigation separates library, downloads and account management. Library emphasizes covers. The download screenshot places downloading, queued and completed sections in a continuous main area with a scrollbar at its outer edge. | Closest overall reference. Related sections can coexist without separately constrained list heights. Do not copy its graph, many sidebar links or controls without a need. |
| [qBittorrent](https://www.qbittorrent.org/screenshots) | Transfers and Execution Log are separate top-level tabs. Transfer details have their own tabs below the selected item; the official gallery shows both collapsed and expanded detail layouts. | Distinguish operating on jobs from inspecting logs. Compact status rows are useful, but a dense permanent split-pane UI is unsuitable as our small-window default. |
| [Zotero](https://www.zotero.org/support/quick_start_guide) | Collections, item list and selected-item metadata occupy distinct panes. Basic search is in the toolbar; advanced search opens separately. | Keep the list focused on recognition and selection; show extensive metadata on demand. Do not require three simultaneous panes at 800px width. |
| [calibre](https://manual.calibre-ebook.com/gui.html#jobs) | Jobs count opens the job list; double-clicking a completed job opens its detailed log. The library supports cover grids and a hideable selected-book detail panel. | Preserve a direct path from failed work to its own diagnostic information. Background work need not occupy permanent large panels while browsing the library. |
| [HandBrake](https://handbrake.fr/docs/en/latest/advanced/queue.html) | Queue management opens a dedicated Queue window. The documented macOS queue shows a main job list and selected-job details. [Activity Log](https://handbrake.fr/docs/en/latest/help/activity-log.html) is separately accessible. | Separate normal job management from troubleshooting. We can use pages/tabs rather than duplicate native windows. |

Useful official images:

- [Heroic library](https://heroicgameslauncher.com/_next/static/images/01-home-350a8d44c674a888040a577e9656fecc.webp)
- [Heroic downloads](https://heroicgameslauncher.com/_next/static/images/03-downloads-767f10f0a009d4fb7702b91574221db2.webp)
- [qBittorrent with selected-transfer details](https://www.qbittorrent.org/img/screenshots/linux/2.webp)
- [HandBrake macOS queue](https://handbrake.fr/docs/en/images/mac/queue-1.10.0.png)

## Proposed direction, pending agreement

- Use Heroic as the primary navigation/content reference. Borrow compact list and
  progressive-detail patterns from the other products, not their complete layouts.
- One primary vertical scroll area per page/tab, spanning the content workspace.
  Avoid independently scrolling cards inside a scrolling page. Dialogs may own
  their scroll while open. Keep headers/controls compact at 800×600.
- Tabs separate different purposes; filters separate states of the same list.
  Related sections can share one naturally growing page. Tabs are not mandatory
  for every screen or every group of rows.
- Give the main list/grid most of the page. Put detailed metadata, account editing
  and diagnostic export options behind deliberate actions; preserve selection,
  filters and scroll position when returning from details.
- Keep failures visible where the user initiated the work, with a path to relevant
  details/logs. Moving diagnostics out of the main view must not hide failures.
- Decide Downloads versus Activity responsibilities before final navigation.
  Avoid maintaining competing primary job lists. Account/settings grouping and
  detail presentation remain open; this research does not approve a menu change.

Next design slice: compare the library, work queue/activity and settings/account
flows together at 800×600 and a larger desktop size, including long lists, failed
jobs, expanded details and returning to a previously scrolled list.
