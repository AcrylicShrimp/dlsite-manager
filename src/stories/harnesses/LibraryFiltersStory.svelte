<script lang="ts">
  import LibraryFilters from "$lib/features/library/LibraryFilters.svelte";
  import type { Account, ProductFilterFacets } from "$lib/model/types";

  let { active = false }: { active?: boolean } = $props();

  const accounts: Account[] = [
    {
      id: "primary",
      label: "Primary",
      loginName: "primary@example.test",
      hasCredential: true,
      enabled: true,
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-05-21T00:00:00Z",
      lastLoginAt: "2026-05-21T00:00:00Z",
      lastSyncAt: "2026-05-21T00:00:00Z",
    },
    {
      id: "archive",
      label: "Archive Account With A Long Label",
      loginName: "archive@example.test",
      hasCredential: true,
      enabled: true,
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-05-21T00:00:00Z",
      lastLoginAt: null,
      lastSyncAt: "2026-05-20T00:00:00Z",
    },
  ];

  const facets: ProductFilterFacets = {
    makers: [
      { name: "North Window Studio", count: 18 },
      { name: "An Especially Long Circle Name For Truncation", count: 7 },
      { name: "Night Signal", count: 4 },
    ],
    customTags: [
      { name: "Favorites", count: 12 },
      { name: "Sleep", count: 8 },
      { name: "Needs Review", count: 3 },
    ],
  };

  let sort = $state("latestPurchaseDesc");
  let selectedAccounts = $state<string[]>([]);
  let selectedSources = $state<string[]>([]);
  let selectedAges = $state<string[]>([]);
  let selectedTypes = $state<string[]>([]);
  let selectedMakers = $state<string[]>([]);
  let selectedTags = $state<string[]>([]);
  let excludedTags = $state<string[]>([]);
  let lastAction = $state("No filter change yet");

  $effect(() => {
    selectedAccounts = active ? ["primary"] : [];
    selectedSources = active ? ["owned"] : [];
    selectedAges = active ? ["all"] : [];
    selectedTypes = active ? ["audio"] : [];
    selectedMakers = active ? ["North Window Studio"] : [];
    selectedTags = active ? ["Favorites"] : [];
    excludedTags = active ? ["Needs Review"] : [];
  });

  function toggle(values: string[], value: string) {
    return values.includes(value) ? values.filter((item) => item !== value) : [...values, value];
  }

  function cycleTag(name: string) {
    if (selectedTags.includes(name)) {
      selectedTags = selectedTags.filter((tag) => tag !== name);
      excludedTags = [...excludedTags, name];
    } else if (excludedTags.includes(name)) {
      excludedTags = excludedTags.filter((tag) => tag !== name);
    } else {
      selectedTags = [...selectedTags, name];
    }
    lastAction = `Cycle ${name}`;
  }
</script>

<main class="story-surface">
  <LibraryFilters
    {accounts}
    {facets}
    {sort}
    selectedAccountIds={selectedAccounts}
    selectedSources={selectedSources}
    selectedAges={selectedAges}
    selectedTypes={selectedTypes}
    selectedMakers={selectedMakers}
    selectedCustomTags={selectedTags}
    excludedCustomTags={excludedTags}
    onSetSort={(value) => { sort = value; lastAction = `Sort: ${value}`; }}
    onClearAccounts={() => { selectedAccounts = []; lastAction = "All accounts"; }}
    onToggleAccount={(value) => { selectedAccounts = toggle(selectedAccounts, value); lastAction = `Account: ${value}`; }}
    onClearSources={() => { selectedSources = []; lastAction = "Any source"; }}
    onToggleSource={(value) => { selectedSources = toggle(selectedSources, value); lastAction = `Source: ${value}`; }}
    onClearAges={() => { selectedAges = []; lastAction = "Any age"; }}
    onToggleAge={(value) => { selectedAges = toggle(selectedAges, value); lastAction = `Age: ${value}`; }}
    onClearTypes={() => { selectedTypes = []; lastAction = "Any type"; }}
    onToggleType={(value) => { selectedTypes = toggle(selectedTypes, value); lastAction = `Type: ${value}`; }}
    onClearMakers={() => { selectedMakers = []; lastAction = "Any maker"; }}
    onToggleMaker={(value) => { selectedMakers = toggle(selectedMakers, value); lastAction = `Maker: ${value}`; }}
    onClearCustomTags={() => { selectedTags = []; excludedTags = []; lastAction = "Any custom tag"; }}
    onCycleCustomTag={cycleTag}
  />
  <p aria-live="polite">{lastAction}</p>
</main>

<style>
  .story-surface {
    width: min(1120px, calc(100vw - 48px));
    margin: 0 auto;
    padding: 24px 0;
  }

  p {
    margin: 12px 0 0;
    color: var(--text-subtle);
    font-size: 12px;
    text-align: right;
  }

  @media (max-width: 720px) {
    .story-surface { width: calc(100vw - 24px); padding: 12px 0; }
  }
</style>
