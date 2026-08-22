<script lang="ts">
  import LibraryView from "$lib/features/library/LibraryView.svelte";
  import type { Account, Product, ProductCreditField } from "$lib/model/types";

  let {
    viewState = "populated",
    initialFiltersOpen = false,
  }: {
    viewState?: "populated" | "loading" | "empty";
    initialFiltersOpen?: boolean;
  } = $props();

  const thumbnail =
    "data:image/svg+xml," +
    encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="240" height="240"><rect width="240" height="240" fill="#203129"/><circle cx="120" cy="92" r="52" fill="#95c29b" fill-opacity=".16"/><path d="M70 157c25-48 75-48 100 0" fill="none" stroke="#95c29b" stroke-width="10" stroke-linecap="round"/><text x="120" y="210" text-anchor="middle" fill="#edf2f6" font-family="system-ui" font-size="20" font-weight="700">RJ SAMPLE</text></svg>`);

  const baseDownload = {
    status: "notDownloaded" as const,
    localPath: null,
    stagingPath: null,
    unpackPolicy: null,
    bytesReceived: 0,
    bytesTotal: null,
    errorCode: null,
    errorMessage: null,
    startedAt: null,
    completedAt: null,
    updatedAt: null,
  };

  const products: Product[] = [
    {
      workId: "RJ01553954",
      title: "A Long Evening at the Observatory — Binaural Voice Drama",
      makerName: "North Window Studio",
      workType: "SOU",
      ageCategory: "all",
      thumbnailUrl: thumbnail,
      publishedAt: "2026-05-05T00:00:00Z",
      updatedAt: "2026-05-18T00:00:00Z",
      earliestPurchasedAt: "2026-05-20T08:30:00Z",
      latestPurchasedAt: "2026-05-21T14:10:00Z",
      creditGroups: [{ kind: "voice", label: "Voice", names: ["Akari Example"] }],
      customTags: [{ name: "Favorites" }, { name: "Sleep" }],
      download: baseDownload,
      owners: [{ accountId: "primary", label: "Primary", purchasedAt: "2026-05-20T08:30:00Z" }],
    },
    {
      workId: "RJ01234567",
      title: "Imported local comic with partial metadata",
      makerName: null,
      workType: "COM",
      ageCategory: "r18",
      thumbnailUrl: null,
      publishedAt: null,
      updatedAt: null,
      earliestPurchasedAt: null,
      latestPurchasedAt: null,
      creditGroups: [],
      customTags: [{ name: "Imported" }],
      download: { ...baseDownload, status: "downloaded", localPath: "/Library/RJ01234567" },
      owners: [{ accountId: "__local__", label: "Local", purchasedAt: null }],
    },
  ];

  const accounts: Account[] = [{
    id: "primary", label: "Primary", loginName: "primary@example.test", hasCredential: true,
    enabled: true, createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-05-21T00:00:00Z",
    lastLoginAt: "2026-05-21T00:00:00Z", lastSyncAt: "2026-05-21T00:00:00Z",
  }];

  let search = $state("");
  let filtersOpen = $state(false);
  let lastAction = $state("No action yet");

  $effect(() => {
    filtersOpen = initialFiltersOpen;
  });

  function action(value: string) { lastAction = value; }
  function productAction(prefix: string, product: Product) { action(`${prefix} ${product.workId}`); }
  function copyCredit(field: ProductCreditField, workId: string) { action(`Copy ${field.label} ${workId}`); }
</script>

<main class="story-surface">
  <LibraryView
    products={viewState === "populated" ? products : []}
    loading={viewState === "loading"}
    bind:search
    {filtersOpen}
    {accounts}
    facets={{ makers: [{ name: "North Window Studio", count: 18 }], customTags: [{ name: "Favorites", count: 12 }] }}
    rangeLabel={viewState === "loading" ? "Loading 248 products" : viewState === "empty" ? "0 products" : "101-102 of 248 products"}
    pageLabel="Page 2 of 3"
    previousDisabled={viewState === "loading"}
    nextDisabled={viewState !== "populated"}
    syncDisabled={viewState === "loading"}
    bulkDisabled={viewState !== "populated"}
    bulkLabel={viewState === "loading" ? "Planning…" : "Download 248 Results"}
    getDownloadLabel={(product) => product.download.status === "downloaded" ? "Open" : "Download"}
    getDownloadTitle={(product) => product.download.localPath ? `Open ${product.download.localPath}` : "Download this work"}
    getDownloadDisabled={() => false}
    onSearch={() => action(`Search: ${search || "all"}`)}
    onReset={() => { search = ""; action("Reset"); }}
    onToggleFilters={() => { filtersOpen = !filtersOpen; action("Toggle filters"); }}
    onReload={() => action("Reload")}
    onSync={() => action("Sync")}
    onBulkDownload={() => action("Bulk download")}
    onSetSort={(value) => action(`Sort ${value}`)}
    onClearAccounts={() => action("All accounts")}
    onToggleAccount={(value) => action(`Account ${value}`)}
    onClearSources={() => action("Any source")}
    onToggleSource={(value) => action(`Source ${value}`)}
    onClearAges={() => action("Any age")}
    onToggleAge={(value) => action(`Age ${value}`)}
    onClearTypes={() => action("Any type")}
    onToggleType={(value) => action(`Type ${value}`)}
    onClearMakers={() => action("Any maker")}
    onToggleMaker={(value) => action(`Maker ${value}`)}
    onClearCustomTags={() => action("Any tag")}
    onCycleCustomTag={(value) => action(`Tag ${value}`)}
    onPreviousPage={() => action("Previous page")}
    onNextPage={() => action("Next page")}
    onPreview={(product) => productAction("Preview", product)}
    onOpenDetails={(product) => productAction("Details", product)}
    onCopyWorkId={(workId) => action(`Copy ${workId}`)}
    onCopyCredit={copyCredit}
    onShowTooltip={(text) => action(text)}
    onMoveTooltip={() => {}}
    onHideTooltip={() => {}}
    onOpenDlsite={(workId) => action(`DLsite ${workId}`)}
    onDownload={(product) => productAction("Download", product)}
    onToggleMenu={(product) => productAction("Menu", product)}
  />
  <p aria-live="polite">{lastAction}</p>
</main>

<style>
  .story-surface { display:grid; gap:12px; width:min(1120px,calc(100vw - 48px)); margin:0 auto; padding:24px 0; }
  p { margin:0; color:var(--text-subtle); font-size:12px; text-align:right; }
  @media (max-width:720px) { .story-surface { width:calc(100vw - 24px); padding:12px 0; } }
</style>
