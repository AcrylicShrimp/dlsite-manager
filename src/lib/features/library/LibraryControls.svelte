<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import TextInput from "$lib/components/ui/TextInput.svelte";

  let {
    search = $bindable(""),
    filtersOpen = false,
    searchDisabled = false,
    reloadDisabled = false,
    syncDisabled = false,
    bulkDisabled = false,
    bulkLabel = "Download Results",
    onSearch,
    onReset,
    onToggleFilters,
    onReload,
    onSync,
    onBulkDownload,
  }: {
    search?: string;
    filtersOpen?: boolean;
    searchDisabled?: boolean;
    reloadDisabled?: boolean;
    syncDisabled?: boolean;
    bulkDisabled?: boolean;
    bulkLabel?: string;
    onSearch?: () => void;
    onReset?: () => void;
    onToggleFilters?: () => void;
    onReload?: () => void;
    onSync?: () => void;
    onBulkDownload?: () => void;
  } = $props();

  function submitSearch(event: SubmitEvent) {
    event.preventDefault();
    onSearch?.();
  }
</script>

<div class="library-controls">
  <form class="search-panel" onsubmit={submitSearch}>
    <div class="search-row">
      <div class="search-input">
        <TextInput
          type="search"
          autocomplete="off"
          spellcheck={false}
          placeholder="Search title, maker, credit, tag, source, work ID"
          ariaLabel="Search library"
          bind:value={search}
        />
      </div>
      <UiButton type="submit" disabled={searchDisabled}>Search</UiButton>
      <UiButton variant="secondary" onclick={onReset}>Reset</UiButton>
      <UiButton
        variant="secondary"
        ariaExpanded={filtersOpen}
        ariaControls="library-filter-grid"
        onclick={onToggleFilters}
      >
        {filtersOpen ? "Hide Filters" : "Show Filters"}
      </UiButton>
    </div>
  </form>

  <div class="actions-panel" aria-label="Library actions">
    <div class="action-group">
      <UiButton variant="secondary" disabled={reloadDisabled} onclick={onReload}>Reload</UiButton>
      <UiButton disabled={syncDisabled} onclick={onSync}>Sync</UiButton>
    </div>
    <div class="action-group bulk-actions">
      <UiButton variant="secondary" disabled={bulkDisabled} onclick={onBulkDownload}>
        {bulkLabel}
      </UiButton>
    </div>
  </div>
</div>

<style>
  .library-controls {
    position: sticky;
    top: 0;
    z-index: 30;
    display: grid;
    flex: 0 0 auto;
    gap: 1px;
    border-bottom: 1px solid var(--border);
    border-radius: 7px 7px 0 0;
    background: var(--border);
    box-shadow: 0 14px 26px rgb(0 0 0 / 22%);
    overflow: hidden;
  }

  .search-panel,
  .actions-panel {
    min-width: 0;
    padding: 14px;
    background: var(--panel-soft);
  }

  .search-row {
    display: grid;
    grid-template-columns: minmax(260px, 1fr) auto auto auto;
    gap: 10px;
    align-items: center;
  }

  .search-input {
    min-width: 0;
  }

  .actions-panel {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px;
    width: 100%;
    padding-block: 10px;
  }

  .action-group {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    min-width: 0;
  }

  .bulk-actions {
    min-width: 128px;
  }

  @media (max-width: 1220px) {
    .actions-panel {
      justify-content: flex-start;
    }
  }

  @media (max-width: 980px) {
    .search-row {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }

    .search-input {
      grid-column: 1 / -1;
    }
  }

  @media (max-width: 720px) {
    .search-row,
    .actions-panel,
    .action-group {
      display: grid;
      grid-template-columns: 1fr;
    }

    .search-input {
      grid-column: auto;
    }

    .action-group:first-child {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
