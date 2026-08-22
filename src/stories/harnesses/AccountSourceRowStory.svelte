<script lang="ts">
  import AccountSourceRow from "$lib/features/accounts/AccountSourceRow.svelte";
  import type { Account } from "$lib/model/types";
  import {
    accountSyncJob,
    disabledAccount,
    primaryAccount,
    syncingAccount,
  } from "../fixtures/accounts";

  let {
    rowState = "selected",
  }: {
    rowState?: "selected" | "syncing" | "disabled";
  } = $props();

  const account = $derived<Account>(
    rowState === "syncing" ? syncingAccount : rowState === "disabled" ? disabledAccount : primaryAccount,
  );
  let lastAction = $state("No action yet");
  function action(value: string) { lastAction = value; }
</script>

<main class="story-surface">
  <AccountSourceRow
    {account}
    selected={rowState === "selected"}
    statusLabel={rowState === "syncing" ? "Loading 120 works" : rowState === "disabled" ? "Disabled" : "Synced"}
    statusTone={rowState === "syncing" ? "syncing" : rowState === "disabled" ? "disabled" : "synced"}
    activeSyncJob={rowState === "syncing" ? accountSyncJob : null}
    onToggleEnabled={(item, enabled) => action(`${enabled ? "Enable" : "Disable"} ${item.id}`)}
    onSelect={(item) => action(`Select ${item.id}`)}
    onSync={(item) => action(`Sync ${item.id}`)}
    onCancelSync={(item) => action(`Cancel ${item.id}`)}
    onRemove={(item) => action(`Remove ${item.id}`)}
  />
  <p aria-live="polite">{lastAction}</p>
</main>

<style>
  .story-surface {
    display: grid;
    gap: 12px;
    width: min(920px, calc(100vw - 48px));
    margin: 24px auto;
  }

  p { margin: 0; color: var(--text-subtle); font-size: 12px; text-align: right; }

  @media (max-width: 720px) {
    .story-surface { width: calc(100vw - 24px); margin: 12px auto; }
  }
</style>
