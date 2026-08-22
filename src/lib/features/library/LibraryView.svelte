<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import type { Account, Product, ProductCreditField, ProductFilterFacets } from "$lib/model/types";
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
    openMenuWorkId = null,
    getDownloadLabel,
    getDownloadTitle,
    getDownloadDisabled,
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
    onPreview,
    onOpenDetails,
    onCopyWorkId,
    onCopyCredit,
    onShowTooltip,
    onMoveTooltip,
    onHideTooltip,
    onOpenDlsite,
    onDownload,
    onToggleMenu,
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
    openMenuWorkId?: string | null;
    getDownloadLabel: (product: Product) => string;
    getDownloadTitle: (product: Product) => string;
    getDownloadDisabled: (product: Product) => boolean;
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
    onPreview: (product: Product) => void;
    onOpenDetails: (product: Product) => void;
    onCopyWorkId: (workId: string) => void;
    onCopyCredit: (field: ProductCreditField, workId: string) => void;
    onShowTooltip: (text: string, event: MouseEvent) => void;
    onMoveTooltip: (text: string, event: MouseEvent) => void;
    onHideTooltip: () => void;
    onOpenDlsite: (workId: string) => void;
    onDownload: (product: Product) => void;
    onToggleMenu: (product: Product, event: MouseEvent) => void;
  } = $props();
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
  {/if}

  <div class="list-header">
    <span>{rangeLabel}</span>
    <div class="pagination-controls" role="navigation" aria-label="Library pages">
      <UiButton size="small" variant="secondary" responsiveWidth="auto" disabled={previousDisabled} onclick={onPreviousPage}>
        Previous
      </UiButton>
      <span>{pageLabel}</span>
      <UiButton size="small" variant="secondary" responsiveWidth="auto" disabled={nextDisabled} onclick={onNextPage}>
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
          detailLoading={detailLoadingWorkId === product.workId}
          downloadLabel={getDownloadLabel(product)}
          downloadTitle={getDownloadTitle(product)}
          downloadDisabled={getDownloadDisabled(product)}
          menuOpen={openMenuWorkId === product.workId}
          {onPreview}
          {onOpenDetails}
          {onCopyWorkId}
          {onCopyCredit}
          {onShowTooltip}
          {onMoveTooltip}
          {onHideTooltip}
          {onOpenDlsite}
          {onDownload}
          {onToggleMenu}
        />
      {/each}
    </div>
  {/if}
</section>

<style>
  .product-area {
    display: flex;
    flex: 0 0 auto;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 16px 40px rgb(0 0 0 / 18%);
    overflow: visible;
  }

  .list-header {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 9px 14px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 13px;
  }

  .pagination-controls {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .pagination-controls > span {
    color: var(--text-subtle);
    font-size: 12px;
    font-weight: 650;
    white-space: nowrap;
  }

  .product-table {
    display: block;
    flex: 0 0 auto;
    min-height: 0;
    overflow: visible;
    overflow-anchor: none;
    overscroll-behavior: contain;
  }

  .empty-state {
    padding: 36px 14px;
    color: var(--muted);
    text-align: center;
  }

  @media (max-width: 720px) {
    .list-header {
      align-items: flex-start;
      flex-direction: column;
    }

    .pagination-controls {
      flex-wrap: wrap;
    }
  }
</style>
