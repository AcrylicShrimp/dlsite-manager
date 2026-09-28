<script lang="ts">
  import PageHeader from "$lib/components/workspace/PageHeader.svelte";
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

<div class="library-controls dm:flex dm:flex-col dm:gap-4">
  <PageHeader title="Library">
    <Button variant="text" disabled={reloadDisabled} onclick={onReload}
      ><Icon name="refresh" />Reload</Button
    >
    <Button variant="primary" disabled={syncDisabled} onclick={onSync}
      ><Icon name="refresh" />Sync</Button
    >
  </PageHeader>
  <form
    onsubmit={submitSearch}
    class="dm:flex dm:flex-wrap dm:items-center dm:gap-2"
  >
    <SearchField
      bind:value={search}
      label="Search library"
      placeholder="Search your collection…"
      clearable
      disabled={searchDisabled}
      onClear={onSearch}
    />
    <Button type="submit" disabled={searchDisabled}>Search</Button>
    <Button
      aria-expanded={filtersOpen}
      aria-controls="library-filter-grid"
      onclick={onToggleFilters}><Icon name="filter" />Filters</Button
    >
  </form>
  <div class="dm:flex dm:items-center dm:justify-between dm:gap-3">
    <Button variant="text" onclick={onReset}>Reset filters</Button>
    <Button variant="text" disabled={bulkDisabled} onclick={onBulkDownload}>
      {bulkLabel} <Icon name="downloads" /></Button
    >
  </div>
</div>
