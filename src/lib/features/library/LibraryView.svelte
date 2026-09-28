<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import type {
    Account,
    Product,
    ProductFilterFacets,
    JobSnapshot,
  } from "$lib/model/types";
  import LibraryControls from "./LibraryControls.svelte";
  import LibraryFilters from "./LibraryFilters.svelte";
  import ProductCard from "./ProductCard.svelte";

  let {
    products = [],
    loading = false,
    search = $bindable(""),
    filtersOpen = false,
    accounts = [],
    facets = { makers: [], customTags: [] },
    sort = "latestPurchaseDesc",
    selectedAccountIds = [],
    selectedSources = [],
    selectedAges = [],
    selectedTypes = [],
    selectedMakers = [],
    selectedCustomTags = [],
    excludedCustomTags = [],
    rangeLabel = "0 products",
    pageLabel = "Page 1 of 1",
    previousDisabled = true,
    nextDisabled = true,
    syncDisabled = false,
    bulkDisabled = false,
    bulkLabel = "Download Results",
    detailLoadingWorkId = null,
    getActiveJob = () => null,
    onSearch,
    onReset,
    onToggleFilters,
    onReload,
    onSync,
    onBulkDownload,
    onSetSort,
    onClearAccounts,
    onToggleAccount,
    onClearSources,
    onToggleSource,
    onClearAges,
    onToggleAge,
    onClearTypes,
    onToggleType,
    onClearMakers,
    onToggleMaker,
    onClearCustomTags,
    onCycleCustomTag,
    onPreviousPage,
    onNextPage,
    onOpenDetails,
  }: {
    products?: Product[];
    loading?: boolean;
    search?: string;
    filtersOpen?: boolean;
    accounts?: Account[];
    facets?: ProductFilterFacets;
    sort?: string;
    selectedAccountIds?: string[];
    selectedSources?: string[];
    selectedAges?: string[];
    selectedTypes?: string[];
    selectedMakers?: string[];
    selectedCustomTags?: string[];
    excludedCustomTags?: string[];
    rangeLabel?: string;
    pageLabel?: string;
    previousDisabled?: boolean;
    nextDisabled?: boolean;
    syncDisabled?: boolean;
    bulkDisabled?: boolean;
    bulkLabel?: string;
    detailLoadingWorkId?: string | null;
    getActiveJob?: (product: Product) => JobSnapshot | null;
    onSearch: () => void;
    onReset: () => void;
    onToggleFilters: () => void;
    onReload: () => void;
    onSync: () => void;
    onBulkDownload: () => void;
    onSetSort: (value: string) => void;
    onClearAccounts: () => void;
    onToggleAccount: (id: string) => void;
    onClearSources: () => void;
    onToggleSource: (value: string) => void;
    onClearAges: () => void;
    onToggleAge: (value: string) => void;
    onClearTypes: () => void;
    onToggleType: (value: string) => void;
    onClearMakers: () => void;
    onToggleMaker: (name: string) => void;
    onClearCustomTags: () => void;
    onCycleCustomTag: (name: string) => void;
    onPreviousPage: () => void;
    onNextPage: () => void;
    onOpenDetails: (product: Product) => void;
  } = $props();
  let results: HTMLDivElement;
  function showResults() {
    results.focus({ preventScroll: true });
    results.scrollIntoView({
      block: "start",
      behavior: matchMedia("(prefers-reduced-motion: reduce)").matches
        ? "instant"
        : "smooth",
    });
  }
</script>

<section class="product-area" aria-label="Library">
  <LibraryControls
    bind:search
    {filtersOpen}
    searchDisabled={loading}
    reloadDisabled={loading}
    {syncDisabled}
    {bulkDisabled}
    {bulkLabel}
    {onSearch}
    {onReset}
    {onToggleFilters}
    {onReload}
    {onSync}
    {onBulkDownload}
  />

  {#if filtersOpen}
    <LibraryFilters
      {accounts}
      {facets}
      {sort}
      {selectedAccountIds}
      {selectedSources}
      {selectedAges}
      {selectedTypes}
      {selectedMakers}
      {selectedCustomTags}
      {excludedCustomTags}
      {onSetSort}
      {onClearAccounts}
      {onToggleAccount}
      {onClearSources}
      {onToggleSource}
      {onClearAges}
      {onToggleAge}
      {onClearTypes}
      {onToggleType}
      {onClearMakers}
      {onToggleMaker}
      {onClearCustomTags}
      {onCycleCustomTag}
    />
    <div class="dm:flex dm:justify-end">
      <UiButton variant="secondary" responsiveWidth="auto" onclick={showResults}
        >Show results ↓</UiButton
      >
    </div>
  {/if}

  <div bind:this={results} class="list-header" tabindex="-1">
    <span>{rangeLabel}</span>
    <div
      class="pagination-controls"
      role="navigation"
      aria-label="Library pages"
    >
      <UiButton
        size="small"
        variant="secondary"
        responsiveWidth="auto"
        disabled={previousDisabled}
        onclick={onPreviousPage}
      >
        Previous
      </UiButton>
      <span>{pageLabel}</span>
      <UiButton
        size="small"
        variant="secondary"
        responsiveWidth="auto"
        disabled={nextDisabled}
        onclick={onNextPage}
      >
        Next
      </UiButton>
    </div>
  </div>

  {#if loading}
    <div class="empty-state">Loading</div>
  {:else if products.length === 0}
    <div class="empty-state">No products</div>
  {:else}
    <div class="product-table" aria-label="Cached products">
      {#each products as product (product.workId)}
        <ProductCard
          {product}
          activeJob={getActiveJob(product)}
          detailLoading={detailLoadingWorkId === product.workId}
          {onOpenDetails}
        />
      {/each}
    </div>
  {/if}
</section>

<style>
  .product-area {
    display: flex;
    flex-direction: column;
    gap: 20px;
    min-width: 0;
  }
  .list-header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    color: var(--muted);
    font-size: 12px;
    outline: none;
  }
  .pagination-controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .product-table {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(165px, 1fr));
    gap: 24px 18px;
    overflow-anchor: none;
  }
  .empty-state {
    padding: 48px 16px;
    text-align: center;
    color: var(--muted);
  }
</style>
