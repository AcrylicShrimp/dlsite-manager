<script lang="ts">
  import LibraryControls from "$lib/features/library/LibraryControls.svelte";

  let {
    initialSearch = "",
    filtersOpen = false,
    searchDisabled = false,
    reloadDisabled = false,
    syncDisabled = false,
    bulkDisabled = false,
    bulkLabel = "Download 24 Results",
  }: {
    initialSearch?: string;
    filtersOpen?: boolean;
    searchDisabled?: boolean;
    reloadDisabled?: boolean;
    syncDisabled?: boolean;
    bulkDisabled?: boolean;
    bulkLabel?: string;
  } = $props();

  let search = $state("");
  let open = $state(false);
  let lastAction = $state("No action yet");

  $effect(() => {
    search = initialSearch;
  });

  $effect(() => {
    open = filtersOpen;
  });
</script>

<main class="story-surface">
  <LibraryControls
    bind:search
    filtersOpen={open}
    {searchDisabled}
    {reloadDisabled}
    {syncDisabled}
    {bulkDisabled}
    {bulkLabel}
    onSearch={() => (lastAction = `Search: ${search || "all products"}`)}
    onReset={() => {
      search = "";
      lastAction = "Reset filters";
    }}
    onToggleFilters={() => {
      open = !open;
      lastAction = open ? "Show filters" : "Hide filters";
    }}
    onReload={() => (lastAction = "Reload products")}
    onSync={() => (lastAction = "Sync accounts")}
    onBulkDownload={() => (lastAction = bulkLabel)}
  />

  {#if open}
    <section id="library-filter-grid" class="filter-placeholder">
      <span>Sort · Purchased</span>
      <span>Age · Any</span>
      <span>Type · Audio</span>
      <span>Source · Owned</span>
    </section>
  {/if}

  <p aria-live="polite">{lastAction}</p>
</main>

<style>
  .story-surface {
    width: min(1120px, calc(100vw - 48px));
    min-height: 360px;
    margin: 0 auto;
    padding: 24px 0;
  }

  .filter-placeholder {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 14px;
    border: 1px solid var(--border);
    border-top: 0;
    background: var(--panel-soft);
  }

  .filter-placeholder span {
    padding: 7px 10px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    color: var(--muted);
    background: var(--panel-raised);
    font-size: 12px;
  }

  p {
    margin: 12px 0 0;
    color: var(--text-subtle);
    font-size: 12px;
    text-align: right;
  }

  @media (max-width: 720px) {
    .story-surface {
      width: calc(100vw - 24px);
      padding: 12px 0;
    }
  }
</style>
