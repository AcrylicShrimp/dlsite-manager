<script lang="ts">
  import PageHeader from "$lib/components/workspace/PageHeader.svelte";
  import Button from "$lib/components/workspace/Button.svelte";
  import Modal from "$lib/components/workspace/Modal.svelte";
  import type { Account, JobSnapshot } from "$lib/model/types";
  import {
    credentialedAccountCount,
    enabledAccountCount,
  } from "$lib/utils/accounts";
  import AccountEditor from "./AccountEditor.svelte";
  import AccountSourceRow from "./AccountSourceRow.svelte";

  type AccountStatusTone =
    "synced" | "syncing" | "failed" | "warning" | "disabled" | "idle";

  let {
    accounts = [],
    loading = false,
    saving = false,
    jobsLoading = false,
    editingAccountId = null,
    label = $bindable(""),
    loginName = $bindable(""),
    password = $bindable(""),
    syncingCount = 0,
    syncAllDisabled = false,
    getActiveSyncJob,
    getStatusLabel,
    getStatusTone,
    onReload,
    onSyncAll,
    onToggleEnabled,
    onEdit,
    onSync,
    onCancelSync,
    onRemove,
    onReset,
    onSave,
  }: {
    accounts?: Account[];
    loading?: boolean;
    saving?: boolean;
    jobsLoading?: boolean;
    editingAccountId?: string | null;
    label?: string;
    loginName?: string;
    password?: string;
    syncingCount?: number;
    syncAllDisabled?: boolean;
    getActiveSyncJob: (accountId: string) => JobSnapshot | null;
    getStatusLabel: (account: Account) => string;
    getStatusTone: (account: Account) => AccountStatusTone;
    onReload: () => void;
    onSyncAll: () => void;
    onToggleEnabled: (account: Account, enabled: boolean) => void;
    onEdit: (account: Account) => void;
    onSync: (account: Account) => void;
    onCancelSync: (account: Account) => void;
    onRemove: (account: Account) => void;
    onReset: () => void;
    onSave: (event: SubmitEvent) => void | boolean | Promise<void | boolean>;
  } = $props();

  let editorOpen = $state(false);
  function edit(account: Account) {
    onEdit(account);
    editorOpen = true;
  }
  function add() {
    onReset();
    editorOpen = true;
  }
  async function save(event: SubmitEvent) {
    const result = await onSave(event);
    if (result !== false) editorOpen = false;
  }
  const selectedAccount = $derived(
    accounts.find((a) => a.id === editingAccountId) ?? null,
  );
</script>

<section class="accounts-panel" aria-label="Accounts">
  <PageHeader title="Accounts"
    ><Button variant="text" disabled={loading || saving} onclick={onReload}
      >Reload</Button
    ><Button
      disabled={loading || jobsLoading || syncAllDisabled}
      onclick={onSyncAll}>Sync all</Button
    ><Button variant="primary" disabled={saving} onclick={add}
      >Add account</Button
    ></PageHeader
  >
  <p class="dm:mb-4 dm:text-sm dm:text-draft-dim">
    {enabledAccountCount(accounts)} enabled · {accounts.length} accounts · {syncingCount}
    syncing
  </p>
  {#if loading}<p
      role="status"
      class="dm:py-10 dm:text-center dm:text-draft-dim"
    >
      Loading…
    </p>{:else if !accounts.length}<p
      class="dm:py-10 dm:text-center dm:text-draft-dim"
    >
      No accounts
    </p>{:else}
    {#each accounts as account (account.id)}<AccountSourceRow
        {account}
        selected={editingAccountId === account.id && editorOpen}
        statusLabel={getStatusLabel(account)}
        statusTone={getStatusTone(account)}
        activeSyncJob={getActiveSyncJob(account.id)}
        {onToggleEnabled}
        onSelect={edit}
        {onSync}
        {onCancelSync}
        {onRemove}
      />{/each}
  {/if}
</section>
{#if editorOpen}
  <Modal
    title={editingAccountId ? "Edit account" : "Add account"}
    dismissible={!saving}
    onClose={() => (editorOpen = false)}
  >
    <AccountEditor
      editing={Boolean(editingAccountId)}
      {saving}
      bind:label
      bind:loginName
      bind:password
      {onReset}
      onSave={save}
    />
    {#if selectedAccount}<div
        class="dm:mt-5 dm:flex dm:items-center dm:justify-between dm:gap-3 dm:border-0 dm:border-t dm:border-solid dm:border-draft-line dm:pt-4"
      >
        <label class="dm:flex dm:items-center dm:gap-2 dm:text-sm"
          ><input
            type="checkbox"
            checked={selectedAccount.enabled}
            disabled={saving || Boolean(getActiveSyncJob(selectedAccount.id))}
            onchange={(e) =>
              selectedAccount &&
              onToggleEnabled(selectedAccount, e.currentTarget.checked)}
          />Enable sync</label
        >
        <Button
          tone="error"
          variant="text"
          disabled={saving || Boolean(getActiveSyncJob(selectedAccount.id))}
          onclick={() => {
            if (selectedAccount) {
              onRemove(selectedAccount);
              editorOpen = false;
            }
          }}>Remove account</Button
        >
      </div>{/if}
  </Modal>
{/if}
