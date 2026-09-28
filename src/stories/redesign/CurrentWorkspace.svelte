<script lang="ts">
  import { onMount } from "svelte";
  import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
  import AppShell from "$lib/components/AppShell.svelte";
  import ProductDetailDialog from "$lib/features/library/ProductDetailDialog.svelte";
  import ProductImagePreview from "$lib/features/library/ProductImagePreview.svelte";
  import {
    emptyFilters,
    queryProducts,
    facetsFor,
    toggle,
    cycleTag,
    makeDetail,
    parseTags,
  } from "./library-preview";
  import LibraryView from "$lib/features/library/LibraryView.svelte";
  import DownloadsView from "$lib/features/downloads/DownloadsView.svelte";
  import ActivityView from "$lib/features/activity/ActivityView.svelte";
  import AccountsView from "$lib/features/accounts/AccountsView.svelte";
  import SettingsView from "$lib/features/settings/SettingsView.svelte";
  import type { View, Account, Product } from "$lib/model/types";
  import {
    makeFixture,
    isActive,
    jobDetail,
    type Scenario,
  } from "../fixtures/redesign";
  let {
    view,
    scenario,
    onNavigate,
  }: { view: View; scenario: Scenario; onNavigate: (view: View) => void } =
    $props();
  const data = $derived(makeFixture(scenario));
  const downloads = $derived(
    data.jobs.filter((j) => j.kind !== "accountSync" && isActive(j)),
  );
  let ready = $state(false);
  let search = $state("");
  let query = $state("");
  let filtersOpen = $state(false);
  let label = $state("Primary DLsite account");
  let loginName = $state("primary@example.test");
  let password = $state("");
  let editingAccountId = $state<string | null>("primary");
  let libraryRoot = $state("/Users/example/Library/DLsite Manager Collection");
  let downloadRoot = $state("/Users/example/Downloads/DLsite Staging");
  let message = $state("");
  let filters = $state(emptyFilters());
  let libraryProducts = $state<Product[]>([]);
  let detailId = $state<string | null>(null);
  let imagePreview = $state<Product | null>(null);
  let customTagInput = $state("");
  const selectedProduct = $derived(
    libraryProducts.find((p) => p.workId === detailId) ?? null,
  );
  const detail = $derived(selectedProduct ? makeDetail(selectedProduct) : null);
  const products = $derived(queryProducts(libraryProducts, query, filters));
  const facets = $derived(facetsFor(libraryProducts, query, filters));
  $effect(() => {
    libraryProducts = data.products;
    filters = emptyFilters();
    search = "";
    query = "";
    detailId = null;
    imagePreview = null;
  });
  $effect(() => {
    view;
    detailId = null;
    imagePreview = null;
  });
  function updateTags(names: string[]) {
    libraryProducts = libraryProducts.map((p) =>
      p.workId === detailId
        ? { ...p, customTags: names.map((name) => ({ name })) }
        : p,
    );
  }
  const action = (value = "Action selected") => {
    message = `Preview: ${value}`;
  };
  const noop = () => {};
  onMount(() => {
    // Isolated to this comparison iframe; no native call escapes the workbench.
    mockIPC((command, args) => {
      if (command === "diagnostic_summary")
        return {
          runId: "run-preview",
          health: {
            state: "available",
            droppedCritical: 0,
            droppedRoutine: 0,
            ringEvicted: 0,
          },
        };
      if (command === "diagnostic_runs")
        return [{ runId: "run-preview", current: true }];
      if (command === "diagnostic_operation")
        return data.events.filter(
          (e) =>
            e.operationId === (args as Record<string, unknown>)?.operationId,
        );
      if (command === "export_diagnostics") {
        action("Export dialog requested — no file written");
        return null;
      }
      throw new Error(`Unsupported preview command: ${command}`);
    });
    ready = true;
    return clearMocks;
  });
  function edit(a: Account) {
    editingAccountId = a.id;
    label = a.label;
    loginName = a.loginName ?? "";
    password = "";
  }
</script>

{#if ready}
  <AppShell activeView={view} {onNavigate}>
    {#if view === "library"}
      <LibraryView
        {products}
        bind:search
        {filtersOpen}
        accounts={data.accounts}
        {facets}
        sort={filters.sort}
        selectedAccountIds={filters.accounts}
        selectedSources={filters.sources}
        selectedAges={filters.ages}
        selectedTypes={filters.types}
        selectedMakers={filters.makers}
        selectedCustomTags={filters.tags}
        excludedCustomTags={filters.excludedTags}
        rangeLabel={`${products.length} products`}
        pageLabel="Page 1 of 1"
        getDownloadLabel={(p) =>
          p.download.status === "downloaded" ? "Open" : "Download"}
        getDownloadTitle={(p) => p.title}
        getDownloadDisabled={() => false}
        onSearch={() => (query = search)}
        onReset={() => {
          search = "";
          query = "";
          filters = emptyFilters();
        }}
        onToggleFilters={() => (filtersOpen = !filtersOpen)}
        onReload={() => action("Reload")}
        onSync={() => action("Sync")}
        onBulkDownload={() => action("Bulk download")}
        onSetSort={(value) => (filters.sort = value)}
        onClearAccounts={() => (filters.accounts = [])}
        onToggleAccount={(value) =>
          (filters.accounts = toggle(filters.accounts, value))}
        onClearSources={() => (filters.sources = [])}
        onToggleSource={(value) =>
          (filters.sources = toggle(filters.sources, value))}
        onClearAges={() => (filters.ages = [])}
        onToggleAge={(value) => (filters.ages = toggle(filters.ages, value))}
        onClearTypes={() => (filters.types = [])}
        onToggleType={(value) => (filters.types = toggle(filters.types, value))}
        onClearMakers={() => (filters.makers = [])}
        onToggleMaker={(value) =>
          (filters.makers = toggle(filters.makers, value))}
        onClearCustomTags={() => {
          filters.tags = [];
          filters.excludedTags = [];
        }}
        onCycleCustomTag={(value) => cycleTag(filters, value)}
        onPreviousPage={noop}
        onNextPage={noop}
        onPreview={(p) => (imagePreview = p)}
        onOpenDetails={(p) => {
          detailId = p.workId;
          customTagInput = "";
        }}
        onCopyWorkId={() => action("Copy ID")}
        onCopyCredit={() => action("Copy credit")}
        onShowTooltip={noop}
        onMoveTooltip={noop}
        onHideTooltip={noop}
        onOpenDlsite={() => action("Open DLsite")}
        onDownload={(p) => action(`Download ${p.title}`)}
        onToggleMenu={() => action("Product menu")}
      />
    {:else if view === "downloads"}
      <DownloadsView
        jobs={downloads}
        queuedCount={downloads.filter((j) => j.status === "queued").length}
        runningCount={downloads.filter((j) => j.status === "running").length}
        getTitle={(j) => j.title}
        getDetail={jobDetail}
        onReload={() => action("Reload")}
        onCancel={(j) => action(`Cancel ${j.title}`)}
      />
    {:else if view === "activity"}
      <ActivityView
        jobs={data.jobs}
        auditEvents={data.events}
        auditLogDir="/Users/example/Library/Logs"
        getJobTitle={(j) => j.title}
        getJobDetail={jobDetail}
        onReloadJobs={() => action("Reload jobs")}
        onClearJobs={() => action("Clear jobs")}
        onCancelJob={(j) => action(`Cancel ${j.title}`)}
        onOpenAuditFolder={() => action("Open log folder")}
        onReloadAudit={() => action("Reload logs")}
      />
    {:else if view === "accounts"}
      <AccountsView
        accounts={data.accounts}
        {editingAccountId}
        bind:label
        bind:loginName
        bind:password
        syncingCount={data.accounts.length ? 1 : 0}
        getActiveSyncJob={(id) =>
          data.jobs.find(
            (j) => j.kind === "accountSync" && j.metadata.accountId === id,
          ) ?? null}
        getStatusLabel={(a) => (a.enabled ? "Connected" : "Disabled")}
        getStatusTone={(a) => (a.enabled ? "synced" : "disabled")}
        onReload={() => action("Reload")}
        onSyncAll={() => action("Sync all")}
        onToggleEnabled={() => action("Toggle account")}
        onEdit={edit}
        onSync={() => action("Sync")}
        onCancelSync={() => action("Cancel sync")}
        onRemove={() => action("Remove account")}
        onReset={() => {
          editingAccountId = null;
          label = "";
          loginName = "";
          password = "";
        }}
        onSave={(e) => {
          e.preventDefault();
          action("Save account");
        }}
      />
    {:else}
      <SettingsView
        bind:libraryRoot
        bind:downloadRoot
        appInfo={{
          name: "DLsite Manager",
          version: "3.4.0",
          identifier: "com.acrylicshrimp.dlsite-manager",
          tauriVersion: "2.11.3",
        }}
        onReload={() => action("Reload settings")}
        onChooseDirectory={() => action("Choose folder")}
        onUseDefaultDownloadRoot={() =>
          (downloadRoot = "/Users/example/Downloads")}
        onSave={(e) => {
          e.preventDefault();
          action("Save settings");
        }}
        onOpenGitHub={() => action("Open GitHub")}
        onOpenDlsite={() => action("Open DLsite")}
        onCheckForUpdates={() => action("No updates available")}
      />
    {/if}
  </AppShell>
{/if}
{#if detail}
  <ProductDetailDialog
    {detail}
    bind:customTagInput
    onClose={() => (detailId = null)}
    onPreview={() => (imagePreview = selectedProduct)}
    onCopyText={(label) => action(`Copy ${label}`)}
    onCopyWorkId={() => action("Copy work ID")}
    onCopyCredit={(field) => action(`Copy ${field.label}`)}
    onOpenDlsite={() => action("Open on DLsite")}
    onAddTags={() => {
      try {
        updateTags(
          parseTags(
            customTagInput,
            detail.customTags.map((t) => t.name),
          ),
        );
        customTagInput = "";
      } catch (e) {
        action(e instanceof Error ? e.message : "Invalid tag");
      }
    }}
    onRemoveTag={(name) =>
      updateTags(
        detail.customTags.filter((t) => t.name !== name).map((t) => t.name),
      )}
  />
{/if}
{#if imagePreview?.thumbnailUrl}<ProductImagePreview
    preview={{
      workId: imagePreview.workId,
      title: imagePreview.title,
      url: imagePreview.thumbnailUrl,
    }}
    onClose={() => (imagePreview = null)}
  />{/if}
{#if message}<button
    class="preview-message"
    onclick={() => (message = "")}
    aria-label="Dismiss preview message">{message} ×</button
  >{/if}

<style>
  .preview-message {
    position: fixed;
    bottom: 14px;
    right: 14px;
    max-width: calc(100% - 28px);
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-raised);
    color: var(--text);
    font-size: 12px;
    z-index: 100;
  }
</style>
