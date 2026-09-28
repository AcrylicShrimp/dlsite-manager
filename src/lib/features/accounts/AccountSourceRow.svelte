<script lang="ts">
  import Button from "$lib/components/workspace/Button.svelte";
  import type { Account, JobSnapshot } from "$lib/model/types";
  import {
    accountCredentialLabel,
    accountEnabledLabel,
    accountLastSyncLabel,
    accountLoginLabel,
  } from "$lib/utils/accounts";

  type AccountStatusTone =
    "synced" | "syncing" | "failed" | "warning" | "disabled" | "idle";

  let {
    account,
    selected = false,
    statusLabel,
    statusTone = "idle",
    activeSyncJob = null,
    onToggleEnabled,
    onSelect,
    onSync,
    onCancelSync,
    onRemove,
  }: {
    account: Account;
    selected?: boolean;
    statusLabel: string;
    statusTone?: AccountStatusTone;
    activeSyncJob?: JobSnapshot | null;
    onToggleEnabled: (account: Account, enabled: boolean) => void;
    onSelect: (account: Account) => void;
    onSync: (account: Account) => void;
    onCancelSync: (account: Account) => void;
    onRemove: (account: Account) => void;
  } = $props();
</script>

<article
  class="account-row dm:flex dm:flex-wrap dm:items-center dm:gap-3 dm:border-0 dm:border-b dm:border-solid dm:border-draft-line dm:px-4 dm:py-4"
>
  <span
    class="dm:grid dm:size-10 dm:shrink-0 dm:place-items-center dm:rounded-lg dm:bg-draft-selected dm:text-draft-accent dm:text-lg"
    >{account.label.slice(0, 1)}</span
  >
  <div
    class="dm:flex dm:min-w-0 dm:flex-1 dm:flex-col dm:gap-1 dm:wrap-anywhere"
  >
    <strong>{account.label}</strong><span class="dm:text-xs dm:text-draft-dim"
      >{accountLoginLabel(account)}</span
    >
    <span
      class={`dm:text-xs ${statusTone === "failed" ? "dm:text-draft-error" : "dm:text-draft-dim"}`}
      >{statusLabel} · {account.enabled ? "Enabled" : "Disabled"}</span
    >
    <span class="dm:text-xs dm:text-draft-dim"
      >Credential: {accountCredentialLabel(account)} · {accountLastSyncLabel(
        account,
      )}</span
    >
  </div>
  <div class="dm:flex dm:flex-wrap dm:gap-2">
    {#if activeSyncJob}<Button
        disabled={!activeSyncJob.cancellable ||
          activeSyncJob.status === "cancelling"}
        onclick={() => onCancelSync(account)}
        >{activeSyncJob.status === "cancelling"
          ? "Cancelling…"
          : "Cancel sync"}</Button
      >{:else}<Button
        disabled={!account.enabled}
        onclick={() => onSync(account)}>Sync</Button
      >{/if}
    <Button variant="text" onclick={() => onSelect(account)}>Edit</Button>
  </div>
</article>
