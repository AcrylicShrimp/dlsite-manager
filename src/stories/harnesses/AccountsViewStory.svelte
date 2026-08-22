<script lang="ts">
  import AccountsView from "$lib/features/accounts/AccountsView.svelte";
  import type { Account, JobSnapshot } from "$lib/model/types";
  import {
    accountSyncJob,
    disabledAccount,
    primaryAccount,
    syncingAccount,
  } from "../fixtures/accounts";

  let {
    viewState = "populated",
  }: {
    viewState?: "populated" | "loading" | "empty";
  } = $props();

  let storyAccounts = $state<Account[]>([primaryAccount, syncingAccount, disabledAccount]);
  let editingAccountId = $state<string | null>(primaryAccount.id);
  let label = $state(primaryAccount.label);
  let loginName = $state(primaryAccount.loginName ?? "");
  let password = $state("");
  let lastAction = $state("No action yet");

  const visibleAccounts = $derived(viewState === "populated" ? storyAccounts : []);

  $effect(() => {
    editingAccountId = viewState === "populated" ? primaryAccount.id : null;
    label = viewState === "populated" ? primaryAccount.label : "";
    loginName = viewState === "populated" ? primaryAccount.loginName ?? "" : "";
    password = "";
  });

  function action(value: string) {
    lastAction = value;
  }

  function edit(account: Account) {
    editingAccountId = account.id;
    label = account.label;
    loginName = account.loginName ?? "";
    password = "";
    action(`Edit ${account.id}`);
  }

  function reset() {
    editingAccountId = null;
    label = "";
    loginName = "";
    password = "";
    action("New account");
  }

  function toggle(account: Account, enabled: boolean) {
    storyAccounts = storyAccounts.map((item) => item.id === account.id ? { ...item, enabled } : item);
    action(`${enabled ? "Enable" : "Disable"} ${account.id}`);
  }

  function activeJob(accountId: string): JobSnapshot | null {
    return accountId === syncingAccount.id ? accountSyncJob : null;
  }

  function statusLabel(account: Account) {
    if (activeJob(account.id)) return "Loading 120 works";
    if (!account.enabled) return "Disabled";
    return account.lastSyncAt ? "Synced" : "Not synced";
  }

  function statusTone(account: Account) {
    if (activeJob(account.id)) return "syncing" as const;
    if (!account.enabled) return "disabled" as const;
    return account.lastSyncAt ? "synced" as const : "idle" as const;
  }
</script>

<main class="story-surface">
  <AccountsView
    accounts={visibleAccounts}
    loading={viewState === "loading"}
    {editingAccountId}
    bind:label
    bind:loginName
    bind:password
    syncingCount={visibleAccounts.filter((account) => activeJob(account.id)).length}
    syncAllDisabled={viewState !== "populated"}
    getActiveSyncJob={activeJob}
    getStatusLabel={statusLabel}
    getStatusTone={statusTone}
    onReload={() => action("Reload")}
    onSyncAll={() => action("Sync all")}
    onToggleEnabled={toggle}
    onEdit={edit}
    onSync={(account) => action(`Sync ${account.id}`)}
    onCancelSync={(account) => action(`Cancel ${account.id}`)}
    onRemove={(account) => action(`Remove ${account.id}`)}
    onReset={reset}
    onSave={(event) => { event.preventDefault(); action(`${editingAccountId ? "Save" : "Add"} ${label}`); }}
  />
  <p aria-live="polite">{lastAction}</p>
</main>

<style>
  .story-surface {
    display: grid;
    gap: 12px;
    width: min(1360px, calc(100vw - 48px));
    margin: 0 auto;
    padding: 24px 0;
  }

  p {
    margin: 0;
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
