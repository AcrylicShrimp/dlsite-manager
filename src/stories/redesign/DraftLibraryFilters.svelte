<script lang="ts">
  import Button from "./DraftButton.svelte";
  import LibraryFilters from "$lib/features/library/LibraryFilters.svelte";
  import type { Account, ProductFilterFacets } from "$lib/model/types";
  import {
    emptyFilters,
    toggle,
    cycleTag,
    filterCount,
    type PreviewFilters,
  } from "./library-preview";
  let {
    filters = $bindable(emptyFilters()),
    accounts,
    facets,
    resultCount,
    onShowResults,
  }: {
    filters: PreviewFilters;
    accounts: Account[];
    facets: ProductFilterFacets;
    resultCount: number;
    onShowResults: () => void;
  } = $props();
</script>

<section class="full-filters" aria-label="Library filters">
  <LibraryFilters
    {accounts}
    {facets}
    sort={filters.sort}
    selectedAccountIds={filters.accounts}
    selectedSources={filters.sources}
    selectedAges={filters.ages}
    selectedTypes={filters.types}
    selectedMakers={filters.makers}
    selectedCustomTags={filters.tags}
    excludedCustomTags={filters.excludedTags}
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
    onToggleMaker={(value) => (filters.makers = toggle(filters.makers, value))}
    onClearCustomTags={() => {
      filters.tags = [];
      filters.excludedTags = [];
    }}
    onCycleCustomTag={(value) => cycleTag(filters, value)}
  />
  <footer>
    <span>Tags: click to include → exclude → clear.</span>
    <div>
      <Button
        variant="secondary"
        disabled={!filterCount(filters) &&
          filters.sort === "latestPurchaseDesc"}
        onclick={() => (filters = emptyFilters())}>Reset filters</Button
      ><Button variant="primary" onclick={onShowResults}
        >Show {resultCount} works ↓</Button
      >
    </div>
  </footer>
</section>

<style>
  .full-filters {
    margin-top: 14px;
    border: 1px solid var(--dm-color-draft-line);
    border-radius: 8px;
    --accent: var(--dm-color-draft-accent);
    --accent-muted: var(--dm-color-draft-selected);
    --text-strong: var(--dm-color-draft-ink);
    --muted: var(--dm-color-draft-dim);
    --field: var(--dm-color-draft-input);
    --danger: var(--dm-color-draft-error);
    --panel-soft: var(--dm-color-draft-surface-muted);
    --text-subtle: var(--dm-color-draft-dim);
    --border: var(--dm-color-draft-line);
    --border-strong: var(--dm-color-draft-line-strong);
  }
  .full-filters :global(.filter-panel) {
    border: 0;
    border-radius: 8px;
    gap: 14px;
  }
  .full-filters :global(.filter-group) {
    grid-template-columns: 85px minmax(0, 1fr);
  }
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px;
    padding: 12px 14px;
    border-top: 1px solid var(--dm-color-draft-line);
    font-size: 11px;
    color: var(--dm-color-draft-dim);
  }
  footer div {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .full-filters :global(.toggle-row button.excluded) {
    border-color: var(--dm-color-draft-error-border);
    color: var(--dm-color-draft-error);
    background: var(--dm-color-draft-error-bg);
  }
  .full-filters :global(.toggle-row button:focus-visible) {
    outline: none;
    box-shadow: var(--dm-shadow-draft-focus-control);
  }
  @media (forced-colors: active) {
    .full-filters :global(.toggle-row button:focus-visible) {
      outline: 1px solid Highlight;
      outline-offset: -2px;
    }
  }
  /* Adapt the preserved production filters to the draft's semantic palette. */
  .full-filters :global(.toggle-row button[data-age-filter]),
  .full-filters :global(.toggle-row button[data-type-filter]),
  .full-filters :global(.toggle-row button[data-source-filter]) {
    --filter-color: var(--dm-color-kind-other-text);
    --filter-soft: var(--dm-color-kind-other-bg);
  }
  .full-filters :global(.toggle-row button[data-age-filter="all"]) {
    --filter-color: var(--dm-color-age-all-text);
    --filter-soft: var(--dm-color-age-all-bg);
  }
  .full-filters :global(.toggle-row button[data-age-filter="r15"]) {
    --filter-color: var(--dm-color-age-r15-text);
    --filter-soft: var(--dm-color-age-r15-bg);
  }
  .full-filters :global(.toggle-row button[data-age-filter="r18"]) {
    --filter-color: var(--dm-color-age-r18-text);
    --filter-soft: var(--dm-color-age-r18-bg);
  }
  .full-filters :global(.toggle-row button[data-type-filter="audio"]) {
    --filter-color: var(--dm-color-kind-audio-text);
    --filter-soft: var(--dm-color-kind-audio-bg);
  }
  .full-filters :global(.toggle-row button[data-type-filter="video"]) {
    --filter-color: var(--dm-color-kind-video-text);
    --filter-soft: var(--dm-color-kind-video-bg);
  }
  .full-filters :global(.toggle-row button[data-type-filter="game"]) {
    --filter-color: var(--dm-color-kind-game-text);
    --filter-soft: var(--dm-color-kind-game-bg);
  }
  .full-filters :global(.toggle-row button[data-type-filter="image"]) {
    --filter-color: var(--dm-color-kind-image-text);
    --filter-soft: var(--dm-color-kind-image-bg);
  }
  .full-filters :global(.toggle-row button[data-type-filter="other"]) {
    --filter-color: var(--dm-color-kind-other-text);
    --filter-soft: var(--dm-color-kind-other-bg);
  }
  .full-filters :global(.toggle-row button[data-source-filter="owned"]) {
    --filter-color: var(--dm-color-source-owned-text);
    --filter-soft: var(--dm-color-source-owned-bg);
  }
  .full-filters :global(.toggle-row button[data-source-filter="localOnly"]) {
    --filter-color: var(--dm-color-source-local-text);
    --filter-soft: var(--dm-color-source-local-bg);
  }
  @media (max-width: 620px) {
    .full-filters :global(.filter-group) {
      grid-template-columns: 1fr;
    }
  }
</style>
