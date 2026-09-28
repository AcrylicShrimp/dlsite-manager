<script lang="ts">
  import PageHeader from "$lib/components/workspace/PageHeader.svelte";
  import PageToolbar from "$lib/components/workspace/PageToolbar.svelte";
  import Button from "$lib/components/workspace/Button.svelte";
  import Icon from "$lib/components/workspace/Icon.svelte";
  import SearchField from "$lib/components/workspace/SearchField.svelte";

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
  <PageHeader title="Library">
    <Button variant="primary" disabled={syncDisabled} onclick={onSync}
      ><Icon name="refresh" />Sync</Button
    >
    {#snippet toolbar()}
      <PageToolbar label="Library tools" {onReload} {reloadDisabled}>
        <form
          onsubmit={submitSearch}
          class="dm:flex dm:min-w-0 dm:flex-1 dm:flex-wrap dm:items-center dm:gap-2"
        >
          <div class="dm:flex dm:min-w-0 dm:basis-48 dm:grow">
            <SearchField
              bind:value={search}
              label="Search library"
              placeholder="Search your collection…"
              clearable
              disabled={searchDisabled}
              onClear={onSearch}
            />
          </div>
          <Button type="submit" disabled={searchDisabled}>Search</Button>
          <Button
            aria-expanded={filtersOpen}
            aria-controls="library-filter-grid"
            onclick={onToggleFilters}><Icon name="filter" />Filters</Button
          >
          <Button variant="text" onclick={onReset}>Reset filters</Button>
        </form>
        {#snippet actions()}
          <Button disabled={bulkDisabled} onclick={onBulkDownload}>
            <Icon name="downloads" />{bulkLabel}</Button
          >
        {/snippet}
      </PageToolbar>
    {/snippet}
  </PageHeader>
</div>
