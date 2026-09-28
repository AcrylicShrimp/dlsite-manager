<script lang="ts">
  import { tick, untrack } from "svelte";
  import { valueOrNull } from "$lib/utils/format";
  import type {
    Account,
    AuditEvent,
    JobSnapshot,
    Product,
    View,
  } from "$lib/model/types";
  import {
    makeFixture,
    labels,
    views,
    isActive,
    progress,
    jobDetail,
    type Scenario,
  } from "../fixtures/redesign";
  import Icon from "./DraftIcon.svelte";
  import Button from "./DraftButton.svelte";
  import AppMark from "./DraftAppMark.svelte";
  import SearchField from "./DraftSearchField.svelte";
  import ChoiceGroup from "./DraftChoiceGroup.svelte";
  import ActivityRows from "./DraftActivityRows.svelte";
  import Sidebar from "./DraftSidebar.svelte";
  import DownloadRow from "./DraftDownloadRow.svelte";
  import AccountRow from "./DraftAccountRow.svelte";
  import DraftLibraryFilters from "./DraftLibraryFilters.svelte";
  import DraftProductDetail from "./DraftProductDetail.svelte";
  import {
    emptyFilters,
    queryProducts,
    facetsFor,
    filterCount,
    makeDetail,
  } from "./library-preview";
  import WorkBadges from "./DraftWorkBadges.svelte";

  let {
    view,
    scenario,
    onNavigate,
  }: { view: View; scenario: Scenario; onNavigate: (view: View) => void } =
    $props();
  const data = $derived(makeFixture(scenario));
  let jobs = $state<JobSnapshot[]>([]);
  let accounts = $state<Account[]>([]);
  let search = $state("");
  let filtersOpen = $state(false);
  let filters = $state(emptyFilters());
  let libraryProducts = $state<Product[]>([]);
  const facets = $derived(facetsFor(libraryProducts, search, filters));
  const activeFilterCount = $derived(filterCount(filters));
  let scrollTop = $state(0);
  let pageHeading: HTMLHeadingElement;
  let resultsHeading = $state<HTMLDivElement>();

  let activityTab = $state("history");
  let activityFilter = $state("all");
  let logSearch = $state("");
  let downloadFilter = $state("all");
  let settingTab = $state("storage");
  let modal = $state<"product" | "job" | "account" | "export" | "bulk" | null>(
    null,
  );
  let product = $state<Product | null>(null);
  let selectedJob = $state<JobSnapshot | null>(null);
  let selectedEvent = $state<AuditEvent | null>(null);
  let accountEnabled = $state(true);
  let accountId = $state<string | null>(null);
  let accountLabel = $state("");
  let loginName = $state("");
  let password = $state("");
  let libraryRoot = $state("/Users/example/Library/DLsite Manager Collection");
  let downloadRoot = $state("/Users/example/Downloads/DLsite Staging");
  let savedLibrary = $state("/Users/example/Library/DLsite Manager Collection");
  let savedDownload = $state("/Users/example/Downloads/DLsite Staging");
  let message = $state("");
  let exportReady = $state(false);
  let exportScope = $state("current");
  let dialog: HTMLDialogElement;
  let scroller: HTMLElement;
  const positions: Partial<Record<View, number>> = {};
  $effect(() => {
    const f = makeFixture(scenario);
    jobs = f.jobs;
    accounts = f.accounts;
    search = "";
    filters = emptyFilters();
    libraryProducts = f.products;
    downloadFilter = "all";
    activityFilter = "all";
    logSearch = "";
    message = "";
  });
  $effect(() => {
    const next = view;
    if (scroller)
      untrack(() => {
        scroller.scrollTop = positions[next] ?? 0;
        scrollTop = scroller.scrollTop;
      });
  });
  $effect(() => {
    view;
    message = "";
    untrack(() => {
      if (dialog?.open) close();
    });
  });
  $effect(() => {
    if (!message) return;
    const timer = setTimeout(() => (message = ""), 4500);
    return () => clearTimeout(timer);
  });
  const activeDownloads = $derived(
    jobs.filter((j) => j.kind !== "accountSync" && isActive(j)),
  );
  const running = $derived(
    activeDownloads.filter((j) => j.status === "running"),
  );
  const queued = $derived(activeDownloads.filter((j) => j.status === "queued"));
  const failures = $derived(jobs.filter((j) => j.status === "failed"));
  const visibleDownloads = $derived(
    activeDownloads.filter(
      (j) => downloadFilter === "all" || j.status === downloadFilter,
    ),
  );
  const history = $derived(
    jobs.filter((j) => !isActive(j) || j.kind === "accountSync"),
  );
  const visibleHistory = $derived(
    history.filter(
      (j) => activityFilter === "all" || j.status === activityFilter,
    ),
  );
  const events = $derived(
    data.events.filter(
      (e) =>
        (activityFilter !== "failed" || e.level === "error") &&
        `${e.message} ${e.operation}`
          .toLowerCase()
          .includes(logSearch.toLowerCase()),
    ),
  );
  const products = $derived(queryProducts(libraryProducts, search, filters));
  const dirty = $derived(
    libraryRoot !== savedLibrary || downloadRoot !== savedDownload,
  );
  function workStatus(p: Product) {
    const job = activeDownloads.find((j) => j.metadata.workId === p.workId);
    if (job?.status === "queued")
      return { icon: "clock", label: "Queued", state: "queued" };
    if (job) {
      const unpacking = job.phase === "unpacking";
      return {
        icon: unpacking ? "unpack" : "downloads",
        label:
          job.status === "cancelling"
            ? "Cancelling"
            : unpacking
              ? "Unpacking"
              : "Downloading",
        state: "active",
      };
    }
    if (p.download.status === "downloaded")
      return { icon: "folder", label: "Available locally", state: "available" };
    if (p.download.status === "downloading")
      return { icon: "downloads", label: "Downloading", state: "active" };
    return null;
  }
  function notify(text: string) {
    message = text;
  }
  async function open(kind: typeof modal) {
    modal = kind;
    message = "";
    await tick();
    dialog.showModal();
  }
  function close() {
    dialog.close();
    modal = null;
  }
  function showJob(j: JobSnapshot) {
    selectedJob = j;
    selectedEvent = null;
    void open("job");
  }
  function showEvent(e: AuditEvent) {
    selectedEvent = e;
    selectedJob = jobs.find((j) => j.id === e.details.jobId) ?? null;
    void open("job");
  }
  function editAccount(a?: Account) {
    accountEnabled = a?.enabled ?? true;
    accountId = a?.id ?? null;
    accountLabel = a?.label ?? "";
    loginName = a?.loginName ?? "";
    password = "";
    void open("account");
  }
  function cancel(j: JobSnapshot) {
    if (!j.cancellable || j.status === "cancelling" || !isActive(j)) return;
    jobs = jobs.map((x) =>
      x.id === j.id ? { ...x, status: "cancelled", cancellable: false } : x,
    );
    libraryProducts = libraryProducts.map((p) =>
      p.workId === j.metadata.workId && p.download.status === "downloading"
        ? { ...p, download: { ...p.download, status: "cancelled" } }
        : p,
    );
    notify("Download cancelled in this preview.");
  }
  function enqueue(p: Product) {
    if (activeDownloads.some((j) => j.metadata.workId === p.workId)) {
      notify("This work is already in the download queue.");
      return;
    }
    jobs = [
      ...jobs,
      {
        id: `preview-${p.workId}`,
        title: p.title,
        kind: "workDownload",
        status: "queued",
        phase: null,
        progress: null,
        metadata: { workId: p.workId },
        output: null,
        error: null,
        cancellable: true,
        createdAt: "2026-09-28T06:00:00Z",
        startedAt: null,
        finishedAt: null,
      },
    ];
    notify("Added to the preview queue.");
  }
  function saveAccount(e: SubmitEvent) {
    e.preventDefault();
    const existing = accounts.find((a) => a.id === accountId);
    const a: Account = {
      id: accountId ?? `preview-account-${accounts.length}`,
      label: accountLabel,
      loginName: valueOrNull(loginName),
      hasCredential:
        Boolean(valueOrNull(password)) || Boolean(existing?.hasCredential),
      enabled: accountEnabled,
      createdAt: existing?.createdAt ?? "2026-09-28T00:00:00Z",
      updatedAt: "2026-09-28T00:00:00Z",
      lastLoginAt: existing?.lastLoginAt ?? null,
      lastSyncAt: existing?.lastSyncAt ?? null,
    };
    accounts = existing
      ? accounts.map((x) => (x.id === a.id ? a : x))
      : [...accounts, a];
    close();
    notify("Account saved in this preview.");
  }
  function updateTags(names: string[]) {
    if (!product) return;
    product = { ...product, customTags: names.map((name) => ({ name })) };
    libraryProducts = libraryProducts.map((p) =>
      p.workId === product?.workId ? product! : p,
    );
  }
  function goToTop() {
    pageHeading.focus({ preventScroll: true });
    scroller.scrollTo({
      top: 0,
      behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches
        ? "instant"
        : "smooth",
    });
  }
  async function showResults() {
    filtersOpen = false;
    await tick();
    resultsHeading?.scrollIntoView({ block: "start" });
    resultsHeading?.focus({ preventScroll: true });
  }
  function failedHistory() {
    activityFilter = "failed";
    activityTab = "history";
    onNavigate("activity");
  }
</script>

<div class="draft-shell">
  <Sidebar
    {view}
    {running}
    queuedCount={queued.length}
    activeCount={activeDownloads.length}
    failureCount={failures.length}
    {onNavigate}
    onFailure={failedHistory}
    onRunning={() => {
      downloadFilter = "running";
      onNavigate("downloads");
    }}
  />

  <main
    class="draft-main"
    bind:this={scroller}
    onscroll={() => {
      positions[view] = scroller.scrollTop;
      scrollTop = scroller.scrollTop;
    }}
    aria-label={`${labels[view]} workspace`}
  >
    <div class="page-content">
      <header class="page-heading">
        <div>
          <h1 bind:this={pageHeading} tabindex="-1" class="dm:outline-none">
            {labels[view]}
          </h1>
        </div>
        <div class="heading-actions">
          {#if view === "library"}<Button
              variant="primary"
              onclick={() => notify("Library sync requested in preview.")}
              ><Icon name="refresh" />Sync library</Button
            >
          {:else if view === "activity"}<Button
              variant="secondary"
              onclick={() => {
                exportScope = "current";
                exportReady = false;
                void open("export");
              }}><Icon name="downloads" />Export diagnostics</Button
            >
          {:else if view === "accounts"}<Button
              variant="primary"
              onclick={() => editAccount()}
              ><Icon name="plus" />Add account</Button
            >
          {:else if view === "downloads"}<Button
              variant="icon"
              class="icon-button"
              aria-label="Refresh downloads"
              onclick={() => notify("Preview queue is up to date.")}
              ><Icon name="refresh" /></Button
            >{/if}
        </div>
      </header>

      {#if view === "library"}
        <div class="toolbar">
          <SearchField
            bind:value={search}
            label="Search library"
            placeholder="Search your collection…"
            clearable
          />
          <Button
            variant="secondary"
            aria-expanded={filtersOpen}
            onclick={() => (filtersOpen = !filtersOpen)}
            ><Icon name="settings" />Filters{activeFilterCount
              ? ` (${activeFilterCount})`
              : ""}</Button
          >
        </div>
        {#if filtersOpen}<DraftLibraryFilters
            bind:filters
            {accounts}
            {facets}
            resultCount={products.length}
            onShowResults={showResults}
          />{/if}
        {#if activeFilterCount}<div class="active-filter-summary">
            <span
              >{activeFilterCount} selected · {filters.tags.length} included tags
              · {filters.excludedTags.length} excluded tags</span
            ><Button
              variant="text"
              class="text-button"
              onclick={() => (filters = emptyFilters())}
              >Clear all filters</Button
            >
          </div>{/if}
        <div
          class="list-meta dm:outline-none"
          bind:this={resultsHeading}
          tabindex="-1"
        >
          <span
            >{products.length} works{activeFilterCount > 0
              ? " · Filtered"
              : ""}</span
          ><Button
            variant="text"
            class="text-button"
            disabled={!products.some((p) => p.download.status !== "downloaded")}
            onclick={() => void open("bulk")}
            >Download results <Icon name="downloads" /></Button
          >
        </div>
        {#if products.length}
          <div class="collection-grid">
            {#each products as p}{@const status = workStatus(p)}
              <article class="work-card">
                <button
                  class="cover dm:draft-focus-image"
                  aria-label={`Details: ${p.title}${status ? ` · ${status.label}` : ""}`}
                  onclick={() => {
                    product = p;
                    void open("product");
                  }}
                >
                  {#if p.thumbnailUrl}<img
                      src={p.thumbnailUrl}
                      alt=""
                    />{:else}<span class="missing-cover">No cover image</span
                    >{/if}
                  <span class="cover-badges"
                    ><WorkBadges
                      workType={p.workType}
                      ageCategory={p.ageCategory}
                    /></span
                  >
                  {#if status}<span
                      class="download-mark"
                      data-state={status.state}
                      title={status.label}
                      role="img"
                      aria-label={status.label}
                      ><Icon name={status.icon} /></span
                    >{/if}
                </button><button
                  class="work-title dm:draft-focus-text"
                  onclick={() => {
                    product = p;
                    void open("product");
                  }}>{p.title}</button
                >
                <p>{p.makerName ?? "Unknown maker"}</p>
              </article>
            {/each}
          </div>
        {:else}<div class="empty">
            <Icon name="library" />
            <h2>
              {data.products.length
                ? "No matching works"
                : "Your collection starts here"}
            </h2>
            <p>
              {data.products.length
                ? "Try another title or clear your filters."
                : "Add an account to bring your purchases together."}
            </p>
            <Button
              variant="secondary"
              onclick={() => {
                if (data.products.length) {
                  search = "";
                  filters = emptyFilters();
                } else onNavigate("accounts");
              }}
              >{data.products.length
                ? "Clear filters"
                : "Manage accounts"}</Button
            >
          </div>{/if}
      {:else if view === "downloads"}
        <ChoiceGroup
          label="Download status"
          value={downloadFilter}
          onchange={(value) => (downloadFilter = value)}
          options={[
            { value: "all", label: "All", count: activeDownloads.length },
            { value: "running", label: "In progress", count: running.length },
            { value: "queued", label: "Queued", count: queued.length },
          ]}
        />
        {#if failures.length}<div class="notice">
            <span class="error-dot"></span><span
              >{failures.length} download couldn’t finish.</span
            ><Button
              variant="text"
              tone="error"
              class="text-button"
              onclick={failedHistory}
              >Review failure <Icon name="arrow" /></Button
            >
          </div>{/if}
        <div class="rows" aria-label="Download jobs">
          {#each visibleDownloads as j}<DownloadRow
              job={j}
              onDetails={() => showJob(j)}
              onCancel={() => cancel(j)}
            />{/each}
        </div>
        {#if !visibleDownloads.length}<div class="empty">
            <Icon name="downloads" />
            <h2>
              No {downloadFilter === "all" ? "active" : downloadFilter} downloads
            </h2>

            <Button variant="secondary" onclick={() => onNavigate("library")}
              >Browse library</Button
            >
          </div>{/if}
        <footer class="list-footer">
          <Button
            variant="text"
            class="text-button"
            onclick={() => {
              activityTab = "history";
              activityFilter = "all";
              onNavigate("activity");
            }}>View history <Icon name="arrow" /></Button
          >
        </footer>
      {:else if view === "activity"}
        <ChoiceGroup
          variant="tabs"
          label="Activity view"
          value={activityTab}
          onchange={(value) => (activityTab = value)}
          options={[
            { value: "history", label: "Work history", count: history.length },
            {
              value: "logs",
              label: "Application logs",
              count: data.events.length,
            },
          ]}
        />
        <div class="toolbar activity-toolbar">
          <ChoiceGroup
            label="Activity filter"
            value={activityFilter}
            onchange={(value) => (activityFilter = value)}
            options={[
              { value: "all", label: "All" },
              { value: "failed", label: "Errors" },
            ]}
          />
          {#if activityTab === "logs"}<SearchField
              bind:value={logSearch}
              label="Search logs"
              placeholder="Search logs…"
              compact
            />{/if}
        </div>
        {#if activityTab === "history"}
          <ActivityRows
            mode="history"
            jobs={visibleHistory}
            onjob={showJob}
            onevent={showEvent}
          />
          {#if !visibleHistory.length}<div class="empty">
              <Icon name="activity" />
              <h2>
                {activityFilter === "failed"
                  ? "No failed work"
                  : "No recent activity"}
              </h2>
            </div>{/if}
        {:else}
          <ActivityRows
            mode="logs"
            {events}
            onjob={showJob}
            onevent={showEvent}
          />
          {#if !events.length}<div class="empty">
              <Icon name="activity" />
              <h2>No matching events</h2>
              <p>Try another search or filter.</p>
            </div>{/if}
        {/if}
      {:else if view === "accounts"}
        <div class="list-meta">
          <span
            >{accounts.filter((a) => a.enabled).length} enabled · {accounts.length}
            accounts</span
          ><Button
            variant="text"
            class="text-button"
            disabled={!accounts.some((a) => a.enabled)}
            onclick={() =>
              notify("Enabled accounts will sync in this preview.")}
            ><Icon name="refresh" />Sync all</Button
          >
        </div>
        <div class="rows">
          {#each accounts as a}<AccountRow
              account={a}
              onEdit={() => editAccount(a)}
            />{/each}
        </div>
        {#if !accounts.length}<div class="empty">
            <Icon name="accounts" />
            <h2>Connect your first account</h2>
            <p>Your purchases will appear in the library after syncing.</p>
            <Button variant="secondary" onclick={() => editAccount()}
              >Add account</Button
            >
          </div>{/if}
      {:else}
        <ChoiceGroup
          variant="tabs"
          label="Settings section"
          value={settingTab}
          onchange={(value) => (settingTab = value)}
          options={[
            { value: "storage", label: "Storage" },
            { value: "about", label: "About & updates" },
          ]}
        />
        {#if settingTab === "storage"}<form
            class="settings-form"
            onsubmit={(e) => {
              e.preventDefault();
              savedLibrary = libraryRoot;
              savedDownload = downloadRoot;
              notify("Storage paths saved in this preview.");
            }}
          >
            <section>
              <h2>Library folder</h2>

              <div class="path-control">
                <label class="path-field dm:focus-within:border-draft-focus"
                  ><Icon name="folder" /><input
                    class="dm:draft-focus-field"
                    aria-label="Library folder"
                    bind:value={libraryRoot}
                    required
                  /></label
                ><Button
                  variant="secondary"
                  type="button"
                  onclick={() =>
                    notify(
                      "Choose a library folder. Native folder selection is simulated in this preview.",
                    )}>Browse</Button
                >
              </div>
            </section>
            <section>
              <h2>Download staging folder</h2>
              <p>Temporary archives and partial downloads.</p>
              <div class="path-control">
                <label class="path-field dm:focus-within:border-draft-focus"
                  ><Icon name="folder" /><input
                    class="dm:draft-focus-field"
                    aria-label="Download staging folder"
                    bind:value={downloadRoot}
                    required
                  /></label
                ><Button
                  variant="secondary"
                  type="button"
                  onclick={() =>
                    notify(
                      "Choose a staging folder. Native folder selection is simulated in this preview.",
                    )}>Browse</Button
                >
              </div>
              <Button
                variant="text"
                type="button"
                class="text-button"
                onclick={() => (downloadRoot = "/Users/example/Downloads")}
                >Use system Downloads folder</Button
              >
            </section>
            <div class="form-footer">
              <span
                >{dirty
                  ? "You have unsaved changes"
                  : "All changes saved"}</span
              ><Button variant="primary" type="submit" disabled={!dirty}
                >Save changes</Button
              >
            </div>
          </form>
        {:else}<section
            class="about dm:max-w-[600px] dm:py-2"
            aria-label="About DLsite Manager"
          >
            <div
              class="dm:flex dm:flex-wrap dm:items-center dm:justify-between dm:gap-4"
            >
              <div
                class="about-identity dm:flex dm:min-w-0 dm:items-center dm:gap-3"
              >
                <AppMark size="large" />
                <div class="dm:min-w-0">
                  <h2
                    class="dm:m-0 dm:text-xl dm:font-semibold dm:leading-tight"
                  >
                    DLsite Manager
                  </h2>
                  <p class="dm:mt-1 dm:text-sm dm:text-draft-dim">
                    Version 3.4.0
                  </p>
                </div>
              </div>
              <Button
                variant="secondary"
                onclick={() =>
                  notify("You’re up to date. Preview result only.")}
                >Check for updates</Button
              >
            </div>
            <div
              class="about-links dm:mt-6 dm:flex dm:flex-wrap dm:items-center dm:gap-4 dm:border-0 dm:border-t dm:border-solid dm:border-draft-line dm:pt-3 dm:text-xs dm:text-draft-dim"
            >
              <Button
                variant="text"
                class="text-button"
                onclick={() =>
                  notify(
                    "GitHub link selected. External navigation is disabled in preview.",
                  )}>GitHub ↗</Button
              ><span>MIT License</span>
            </div>
          </section>{/if}
      {/if}
    </div>
  </main>
</div>

<dialog
  class:product-dialog={modal === "product"}
  bind:this={dialog}
  onclose={() => (modal = null)}
  aria-labelledby="draft-dialog-title"
>
  <div class="modal-heading">
    <h2 id="draft-dialog-title">
      {modal === "product"
        ? "Work details"
        : modal === "job"
          ? "Activity details"
          : modal === "account"
            ? accountId
              ? "Edit account"
              : "Add account"
            : modal === "bulk"
              ? "Download results"
              : "Export diagnostics"}
    </h2>
    <Button
      variant="icon"
      class="icon-button"
      aria-label="Close dialog"
      onclick={close}><Icon name="close" /></Button
    >
  </div>
  <div class="modal-body">
    {#if modal === "product" && product}
      <DraftProductDetail
        detail={makeDetail(product)}
        activeJob={activeDownloads.find(
          (j) => j.metadata.workId === product?.workId,
        ) ?? null}
        onTags={updateTags}
        onDownload={() => {
          if (product) {
            if (product.download.status === "downloaded")
              notify("Open folder requested. No native action in preview.");
            else enqueue(product);
          }
        }}
        onAction={(action) =>
          notify(
            `${action} requested for ${product?.workId}. Preview only; no native changes.`,
          )}
      />
      <div class="modal-footer">
        <Button
          variant="secondary"
          onclick={() => dialog.scrollTo({ top: 0, behavior: "smooth" })}
          >Back to top ↑</Button
        >
      </div>
    {:else if modal === "job"}
      <span
        class="status"
        class:error-text={selectedJob?.status === "failed" ||
          selectedEvent?.level === "error"}
        >{selectedEvent?.outcome ?? selectedJob?.status}</span
      >
      <h3>{selectedEvent?.message ?? selectedJob?.title}</h3>
      <p>
        {selectedEvent?.errorMessage ??
          (selectedJob
            ? jobDetail(selectedJob)
            : "Operation finished successfully.")}
      </p>
      <dl>
        <dt>Operation</dt>
        <dd>{selectedEvent?.operation ?? selectedJob?.kind}</dd>
        <dt>Reference</dt>
        <dd>{selectedEvent?.operationId ?? selectedJob?.id}</dd>
      </dl>
      <div class="modal-footer">
        <Button variant="secondary" onclick={close}>Close</Button><Button
          variant="primary"
          onclick={() => {
            exportScope = "operation";
            exportReady = false;
            modal = "export";
          }}>Export related diagnostics</Button
        >
      </div>
    {:else if modal === "account"}
      <form class="editor" onsubmit={saveAccount}>
        <label
          >Account name<input
            class="dm:draft-focus-field"
            bind:value={accountLabel}
            required
            placeholder="e.g. Personal"
          /></label
        ><label
          >Login<input
            class="dm:draft-focus-field"
            bind:value={loginName}
            autocomplete="username"
            spellcheck={false}
          /></label
        ><label
          >Password<input
            class="dm:draft-focus-field"
            type="password"
            bind:value={password}
            placeholder={accountId ? "Leave blank to keep saved password" : ""}
            autocomplete="new-password"
          /></label
        >{#if accountId}<label class="check-field"
            ><input
              class="dm:draft-focus-control"
              type="checkbox"
              bind:checked={accountEnabled}
            /> Include this account in sync</label
          >{/if}
        <p class="footnote">Sample data only. Don’t enter real credentials.</p>
        <div class="modal-footer">
          <Button variant="secondary" type="button" onclick={close}
            >Cancel</Button
          ><Button variant="primary" type="submit">Save account</Button>
        </div>
      </form>
    {:else if modal === "export"}
      <label class="export-label"
        >Include<select class="dm:draft-focus-field" bind:value={exportScope}
          ><option value="current">Current app session</option><option
            value="operation"
            disabled={!selectedJob && !selectedEvent}>Selected operation</option
          ></select
        ></label
      >
      {#if exportReady}<p class="export-result" role="status">
          Preview complete. The app would open a save dialog for a ZIP file. No
          file was written.
        </p>{/if}
      <div class="modal-footer">
        <Button variant="secondary" onclick={close}>Cancel</Button><Button
          variant="primary"
          onclick={() => (exportReady = true)}>Export ZIP</Button
        >
      </div>
    {:else if modal === "bulk"}
      <p>
        Add {products.filter(
          (p) =>
            p.download.status !== "downloaded" &&
            !activeDownloads.some((j) => j.metadata.workId === p.workId),
        ).length} works to the download queue? Downloaded and already queued works
        will be skipped.
      </p>
      <div class="modal-footer">
        <Button variant="secondary" onclick={close}>Cancel</Button><Button
          variant="primary"
          onclick={() => {
            for (const p of products.filter(
              (p) => p.download.status !== "downloaded",
            ))
              enqueue(p);
            close();
            notify("Works added to the preview queue.");
          }}>Add to queue</Button
        >
      </div>
    {/if}
    {#if message}<p role="status">{message}</p>{/if}
  </div>
</dialog>
{#if scrollTop > 300 && !modal && !message}<button
    class="back-to-top dm:draft-focus-control"
    aria-label="Back to top"
    title="Back to top"
    onclick={goToTop}><Icon name="up" /></button
  >{/if}
{#if message && !modal}<div class="toast" role="status">
    <span>{message}</span><Button
      variant="icon"
      class="icon-button"
      aria-label="Dismiss notification"
      onclick={() => (message = "")}><Icon name="close" /></Button
    >
  </div>{/if}

<style>
  .draft-shell,
  dialog,
  .toast {
    --surface: var(--dm-color-draft-surface-muted);
    --line: var(--dm-color-draft-line-subtle);
    --ink: var(--dm-color-draft-ink);
    --dim: var(--dm-color-draft-dim);
    --green: var(--dm-color-draft-accent);
    color: var(--ink);
    font-size: 13px;
    line-height: 1.5;
  }
  .product-dialog {
    width: min(760px, calc(100vw - 32px));
  }
  .back-to-top {
    position: fixed;
    right: 24px;
    bottom: 24px;
    z-index: 25;
    background: var(--dm-color-draft-selected-strong);
    color: var(--dm-color-draft-ink);
    border: 1px solid var(--dm-color-draft-selected-border);
    box-shadow: 0 4px 16px var(--dm-color-draft-shadow);
    width: 40px;
    height: 40px;
    min-height: 40px;
    padding: 0;
    border-radius: 50%;
  }
  .active-filter-summary {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    align-items: center;
    margin-top: 12px;
    color: var(--dm-color-draft-dim);
    font-size: 11px;
  }
  .missing-cover {
    display: grid;
    place-items: center;
    min-height: 100%;
    color: var(--dm-color-draft-dim);
    font-size: 11px;
  }
  .draft-shell {
    display: grid;
    grid-template-columns: 194px minmax(0, 1fr);
    height: 100vh;
    overflow: hidden;
    background: var(--dm-color-draft-background);
  }
  button,
  input,
  select {
    font: inherit;
  }
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    min-height: 34px;
    padding: 6px 12px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: var(--dm-color-draft-elevated);
    color: var(--ink);
    cursor: pointer;
    text-align: left;
    transition: background 0.12s;
  }
  button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  h1,
  h2,
  h3,
  p {
    margin: 0;
  }
  h1 {
    font-size: 26px;
    line-height: 1.2;
    font-weight: 650;
    letter-spacing: -0.7px;
  }
  h2 {
    font-size: 16px;
    font-weight: 600;
  }
  h3 {
    font-size: 20px;
    line-height: 1.4;
    margin-top: 16px;
  }
  p {
    color: var(--dim);
  }
  .draft-main {
    min-width: 0;
    min-height: 0;
    overflow: auto;
    scrollbar-gutter: stable;
  }
  .page-content {
    max-width: 1240px;
    margin: auto;
    padding: 32px 32px 40px;
  }
  .page-heading {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: center;
    margin-bottom: 28px;
  }
  .heading-actions {
    display: flex;
    flex: none;
    gap: 8px;
  }
  .toolbar {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  input {
    min-width: 0;
    color: var(--ink);
    background: transparent;
    border: 0;
    outline-offset: 0;
  }
  input::placeholder {
    color: var(--dm-color-draft-placeholder);
  }
  .list-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin: 16px 0;
    color: var(--dim);
    font-size: 11px;
  }
  .collection-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(172px, 1fr));
    gap: 26px 18px;
  }
  .work-card {
    min-width: 0;
  }
  .cover {
    position: relative;
    display: block;
    padding: 0;
    border: 0;
    width: 100%;
    aspect-ratio: 3/2;
    overflow: hidden;
    border-radius: 7px;
    background: var(--dm-color-draft-elevated);
  }
  .cover img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform 0.2s;
  }
  .cover:hover img {
    transform: scale(1.035);
  }
  .cover-badges {
    position: absolute;
    left: 8px;
    right: 8px;
    bottom: 8px;
    display: flex;
    text-align: left;
  }
  .download-mark {
    position: absolute;
    right: 9px;
    top: 9px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    background: var(--dm-color-draft-status-bg);
    border-radius: 50%;
    color: var(--dm-color-draft-available);
  }
  .download-mark[data-state="queued"] {
    color: var(--dm-color-draft-queued);
  }
  .download-mark[data-state="active"] {
    color: var(--dm-color-draft-in-progress);
  }
  .work-title {
    display: block;
    padding: 0;
    margin-top: 11px;
    min-height: 0;
    background: none;
    border: 0;
    border-radius: 0;
    font-size: 13px;
    line-height: 1.5;
    font-weight: 550;
  }
  .work-title:hover {
    color: var(--green);
    background: none;
  }
  .work-card p {
    font-size: 11px;
    margin-top: 3px;
  }
  .notice {
    display: flex;
    gap: 9px;
    align-items: center;
    background: var(--dm-color-draft-error-bg);
    border: 1px solid var(--dm-color-draft-error-border);
    border-radius: 6px;
    padding: 8px 12px;
    margin: 20px 0 6px;
    font-size: 12px;
    color: var(--dm-color-draft-error);
  }
  .notice :global(.text-button) {
    margin-left: auto;
  }
  .error-dot {
    width: 6px;
    height: 6px;
    flex: none;
    border-radius: 50%;
    background: var(--dm-color-draft-error);
  }
  .rows {
    margin-top: 8px;
  }
  .status {
    color: var(--dim);
    font-size: 10px;
  }
  .list-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 24px;
    color: var(--dim);
    font-size: 11px;
  }
  .activity-toolbar {
    justify-content: space-between;
    margin-bottom: 10px;
    font-size: 10px;
  }
  .activity-toolbar :global(.search-field) {
    max-width: 230px;
  }
  .error-text {
    color: var(--dm-color-draft-error);
  }
  .footnote {
    margin-top: 22px;
    font-size: 11px;
  }
  .settings-form {
    max-width: 720px;
  }
  .settings-form section {
    padding: 20px 0 27px;
    border-bottom: 1px solid var(--line);
  }
  .settings-form h2 {
    font-size: 14px;
    margin-bottom: 12px;
  }
  .settings-form p {
    font-size: 12px;
    margin: 6px 0 16px;
    max-width: 480px;
  }
  .path-field {
    display: flex;
    align-items: center;
    gap: 10px;
    border: 1px solid var(--line);
    padding: 10px 12px;
    border-radius: 6px;
    background: var(--dm-color-draft-input);
    color: var(--dim);
    min-width: 0;
  }
  .path-control {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
  }
  .path-field input {
    width: 100%;
    font-size: 12px;
  }
  .settings-form :global(.text-button) {
    margin-top: 9px;
  }
  .form-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding-top: 23px;
  }
  .form-footer > span {
    font-size: 11px;
    color: var(--dim);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    min-height: 280px;
    text-align: center;
    color: var(--dim);
    padding: 30px 15px;
  }
  .empty > :global(svg) {
    width: 30px;
    height: 30px;
    color: var(--green);
    margin-bottom: 6px;
  }
  .empty h2 {
    color: var(--ink);
  }
  .empty p {
    max-width: 300px;
    font-size: 12px;
  }
  dialog {
    position: fixed;
    background: var(--dm-color-draft-surface);
    border: 1px solid var(--dm-color-draft-line-strong);
    border-radius: 12px;
    width: min(510px, calc(100vw - 32px));
    max-height: calc(100dvh - 40px);
    padding: 0;
    overflow: auto;
    box-shadow: 0 24px 100px var(--dm-color-draft-shadow-strong);
  }
  dialog::backdrop {
    background: var(--dm-color-draft-backdrop);
    backdrop-filter: blur(3px);
  }
  .modal-heading {
    position: sticky;
    top: 0;
    z-index: 2;
    padding: 14px 24px;
    border-bottom: 1px solid var(--dm-color-draft-line);
    background: var(--dm-color-draft-surface);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .modal-body {
    padding: 20px 24px 24px;
  }
  .modal-heading h2 {
    font-size: 16px;
  }
  dialog p {
    font-size: 12px;
    margin-top: 8px;
  }
  dl {
    display: grid;
    grid-template-columns: 100px minmax(0, 1fr);
    gap: 12px;
    font-size: 12px;
    margin: 25px 0;
  }
  dt {
    color: var(--dim);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 26px;
  }
  .editor {
    display: grid;
    gap: 17px;
  }
  .editor label {
    display: grid;
    gap: 7px;
    font-size: 12px;
  }
  .editor input,
  .export-label select {
    width: 100%;
    padding: 10px;
    border: 1px solid
      var(--draft-field-border, var(--dm-color-draft-line-strong));
    border-radius: 6px;
    background: var(--dm-color-draft-input);
    color: var(--ink);
  }
  .editor .check-field {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .check-field input {
    width: auto;
  }
  .editor .footnote {
    margin: 0;
  }
  .editor .modal-footer {
    margin-top: 5px;
  }
  .export-label {
    display: grid;
    gap: 8px;
    font-size: 12px;
    margin-top: 20px;
  }
  .export-result {
    padding: 12px;
    border: 1px solid var(--line);
    border-radius: 6px;
  }
  .toast {
    position: fixed;
    right: 20px;
    bottom: 20px;
    z-index: 100;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px 10px 16px;
    max-width: min(430px, calc(100vw - 40px));
    border: 1px solid var(--dm-color-draft-line-strong);
    border-radius: 8px;
    background: var(--dm-color-draft-selected);
    box-shadow: 0 8px 28px var(--dm-color-draft-shadow);
    font-size: 12px;
  }
  @media (min-width: 1100px) {
    .page-content {
      padding: 40px 48px;
    }
    .draft-shell {
      grid-template-columns: 216px minmax(0, 1fr);
    }
  }
  @media (max-width: 900px) {
    .page-content {
      padding: 26px 24px;
    }
    .collection-grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 23px 14px;
    }
    .page-heading {
      margin-bottom: 23px;
    }
  }
  @media (max-width: 620px) {
    .draft-shell {
      grid-template-columns: 1fr;
      grid-template-rows: auto minmax(0, 1fr);
    }
    .page-content {
      padding: 22px 18px 30px;
    }
    h1 {
      font-size: 24px;
    }
    .page-heading {
      flex-wrap: wrap;
      gap: 12px;
      margin-bottom: 20px;
    }
    .heading-actions {
      margin-left: auto;
    }
    .collection-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .download-mark[data-state="queued"] {
      color: var(--dm-color-draft-queued);
    }
    .download-mark[data-state="active"] {
      color: var(--dm-color-draft-in-progress);
    }
    .work-title {
      font-size: 12px;
    }
    .notice {
      flex-wrap: wrap;
    }
    .notice :global(.text-button) {
      margin-left: 15px;
    }
    .list-footer {
      align-items: start;
      flex-direction: column;
    }
    .activity-toolbar {
      gap: 8px;
    }
    .activity-toolbar :global(.search-field) {
      max-width: 185px;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    button,
    .cover img {
      transition: none;
    }
  }
</style>
