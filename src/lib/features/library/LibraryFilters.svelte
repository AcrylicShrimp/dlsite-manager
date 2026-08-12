<script lang="ts">
  import { AGE_FILTERS, SORT_OPTIONS, SOURCE_FILTERS, TYPE_FILTERS } from "$lib/model/constants";
  import type { Account, ProductFilterFacets } from "$lib/model/types";

  let {
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
  }: {
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
    onSetSort?: (value: string) => void;
    onClearAccounts?: () => void;
    onToggleAccount?: (id: string) => void;
    onClearSources?: () => void;
    onToggleSource?: (value: string) => void;
    onClearAges?: () => void;
    onToggleAge?: (value: string) => void;
    onClearTypes?: () => void;
    onToggleType?: (value: string) => void;
    onClearMakers?: () => void;
    onToggleMaker?: (name: string) => void;
    onClearCustomTags?: () => void;
    onCycleCustomTag?: (name: string) => void;
  } = $props();

  function customTagState(name: string) {
    if (selectedCustomTags.includes(name)) return "include";
    if (excludedCustomTags.includes(name)) return "exclude";
    return "none";
  }
</script>

<div id="library-filter-grid" class="filter-panel">
  <div class="filter-group sort-filter">
    <span>Sort</span>
    <div class="toggle-row">
      {#each SORT_OPTIONS as [value, label] (value)}
        <button class:active={sort === value} type="button" onclick={() => onSetSort?.(value)}>
          <span class="filter-chip-label">{label}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="filter-group">
    <span>Accounts</span>
    <div class="toggle-row">
      <button class:active={selectedAccountIds.length === 0} type="button" onclick={onClearAccounts}>
        <span class="filter-chip-label">All</span>
      </button>
      {#each accounts as account (account.id)}
        <button
          class:active={selectedAccountIds.includes(account.id)}
          type="button"
          title={account.loginName ?? account.label}
          onclick={() => onToggleAccount?.(account.id)}
        >
          <span class="filter-chip-label">{account.label}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="filter-group">
    <span>Source</span>
    <div class="toggle-row">
      <button class:active={selectedSources.length === 0} type="button" onclick={onClearSources}>
        <span class="filter-chip-label">Any</span>
      </button>
      {#each SOURCE_FILTERS as [value, label] (value)}
        <button
          class:active={selectedSources.includes(value)}
          data-source-filter={value}
          type="button"
          onclick={() => onToggleSource?.(value)}
        >
          <span class="filter-chip-label">{label}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="filter-group">
    <span>Age</span>
    <div class="toggle-row">
      <button class:active={selectedAges.length === 0} type="button" onclick={onClearAges}>
        <span class="filter-chip-label">Any</span>
      </button>
      {#each AGE_FILTERS as [value, label] (value)}
        <button
          class:active={selectedAges.includes(value)}
          data-age-filter={value}
          type="button"
          onclick={() => onToggleAge?.(value)}
        >
          <span class="filter-chip-label">{label}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="filter-group">
    <span>Type</span>
    <div class="toggle-row">
      <button class:active={selectedTypes.length === 0} type="button" onclick={onClearTypes}>
        <span class="filter-chip-label">Any</span>
      </button>
      {#each TYPE_FILTERS as [value, label] (value)}
        <button
          class:active={selectedTypes.includes(value)}
          data-type-filter={value}
          type="button"
          onclick={() => onToggleType?.(value)}
        >
          <span class="filter-chip-label">{label}</span>
        </button>
      {/each}
    </div>
  </div>

  <div class="filter-group maker-filter">
    <span>Makers</span>
    <div class="toggle-row">
      <button class:active={selectedMakers.length === 0} type="button" onclick={onClearMakers}>
        <span class="filter-chip-label">Any</span>
      </button>
      {#each facets.makers as maker (maker.name)}
        <button
          class:active={selectedMakers.includes(maker.name)}
          type="button"
          title={`${maker.name} (${maker.count})`}
          onclick={() => onToggleMaker?.(maker.name)}
        >
          <span class="filter-chip-label">{maker.name}</span>
          <small>{maker.count}</small>
        </button>
      {/each}
    </div>
  </div>

  <div class="filter-group custom-tag-filter">
    <span>Custom Tags</span>
    <div class="toggle-row">
      <button
        class:active={selectedCustomTags.length === 0 && excludedCustomTags.length === 0}
        type="button"
        onclick={onClearCustomTags}
      >
        <span class="filter-chip-label">Any</span>
      </button>
      {#each facets.customTags as tag (tag.name)}
        {@const state = customTagState(tag.name)}
        <button
          class:active={state === "include"}
          class:excluded={state === "exclude"}
          type="button"
          title={state === "include"
            ? `Including ${tag.name}. Click to exclude.`
            : state === "exclude"
              ? `Excluding ${tag.name}. Click to clear.`
              : `Click to include ${tag.name}; click again to exclude.`}
          onclick={() => onCycleCustomTag?.(tag.name)}
        >
          <span class="filter-chip-label">{state === "exclude" ? `Not ${tag.name}` : tag.name}</span>
          <small>{tag.count}</small>
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .filter-panel {
    display: grid;
    flex: 0 0 auto;
    gap: 10px;
    min-width: 0;
    padding: 14px;
    border-bottom: 1px solid var(--border);
    background: var(--panel-soft);
  }

  .filter-group {
    display: grid;
    grid-template-columns: 78px minmax(0, 1fr);
    gap: 10px;
    align-items: start;
    min-width: 0;
  }

  .filter-group > span {
    padding-top: 6px;
    color: var(--text-subtle);
    font-size: 12px;
    font-weight: 700;
  }

  .toggle-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
  }

  .maker-filter .toggle-row {
    align-items: flex-start;
  }

  .toggle-row button {
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
    min-width: 0;
    max-width: 210px;
    height: 30px;
    padding: 0 10px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    color: var(--muted);
    background: var(--field);
    font: inherit;
    font-size: 12px;
    font-weight: 650;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    transition:
      border-color 120ms ease,
      background-color 120ms ease,
      color 120ms ease;
    white-space: nowrap;
  }

  .toggle-row button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .toggle-row button.active {
    border-color: var(--accent);
    color: var(--text-strong);
    background: var(--accent-muted);
  }

  .toggle-row button[data-age-filter],
  .toggle-row button[data-source-filter],
  .toggle-row button[data-type-filter] {
    --filter-color: #8b949e;
    --filter-soft: rgb(139 148 158 / 12%);
    border-color: color-mix(in srgb, var(--filter-color) 22%, var(--border-strong));
    color: color-mix(in srgb, var(--filter-color) 28%, var(--text-subtle));
    background: color-mix(in srgb, var(--filter-soft) 24%, var(--field));
  }

  .toggle-row button[data-age-filter]:hover:not(:disabled),
  .toggle-row button[data-source-filter]:hover:not(:disabled),
  .toggle-row button[data-type-filter]:hover:not(:disabled) {
    border-color: color-mix(in srgb, var(--filter-color) 72%, var(--border-strong));
    color: color-mix(in srgb, var(--filter-color) 82%, var(--text-strong));
  }

  .toggle-row button[data-age-filter].active,
  .toggle-row button[data-source-filter].active,
  .toggle-row button[data-type-filter].active {
    border-color: color-mix(in srgb, var(--filter-color) 82%, white);
    color: var(--text-strong);
    background: color-mix(in srgb, var(--filter-color) 28%, var(--field));
  }

  .toggle-row button[data-age-filter="all"],
  .toggle-row button[data-source-filter="owned"] { --filter-color: #9bc89f; --filter-soft: rgb(112 165 120 / 14%); }
  .toggle-row button[data-age-filter="r15"] { --filter-color: #d2b56c; --filter-soft: rgb(204 166 61 / 14%); }
  .toggle-row button[data-age-filter="r18"] { --filter-color: #d77b7b; --filter-soft: rgb(185 64 64 / 16%); }
  .toggle-row button[data-source-filter="localOnly"] { --filter-color: #64b5d9; --filter-soft: rgb(100 181 217 / 14%); }
  .toggle-row button[data-type-filter="audio"] { --filter-color: #d8a62d; --filter-soft: rgb(216 166 45 / 14%); }
  .toggle-row button[data-type-filter="video"] { --filter-color: #d64b92; --filter-soft: rgb(214 75 146 / 14%); }
  .toggle-row button[data-type-filter="game"] { --filter-color: #9863df; --filter-soft: rgb(152 99 223 / 15%); }
  .toggle-row button[data-type-filter="image"] { --filter-color: #4fb85b; --filter-soft: rgb(79 184 91 / 14%); }
  .toggle-row button[data-type-filter="other"] { --filter-color: #8b949e; --filter-soft: rgb(139 148 158 / 12%); }

  .toggle-row button.excluded {
    border-color: rgb(248 113 113 / 52%);
    color: #fca5a5;
    background: rgb(248 113 113 / 12%);
  }

  .filter-chip-label {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .toggle-row button small {
    flex: 0 0 auto;
    margin-left: 6px;
    color: var(--text-subtle);
    font-size: 11px;
    font-weight: 700;
  }

  @media (max-width: 720px) {
    .filter-group {
      grid-template-columns: 1fr;
    }

    .filter-group > span {
      padding-top: 0;
    }

    .toggle-row button {
      flex: 1 1 130px;
    }
  }
</style>
