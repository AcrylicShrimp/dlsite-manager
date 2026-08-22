<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import type { Account, JobSnapshot } from "$lib/model/types";
  import { credentialedAccountCount, enabledAccountCount } from "$lib/utils/accounts";
  import AccountEditor from "./AccountEditor.svelte";
  import AccountSourceRow from "./AccountSourceRow.svelte";

  type AccountStatusTone = "synced" | "syncing" | "failed" | "warning" | "disabled" | "idle";

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
    onSave: (event: SubmitEvent) => void;
  } = $props();
</script>

<div class="accounts-layout">
  <section class="accounts-panel" aria-label="Accounts">
    <div class="panel-title">
      <div>
        <h2>Account sources</h2>
        <p>{enabledAccountCount(accounts)} enabled of {accounts.length}</p>
      </div>
      <div class="panel-actions">
        <UiButton
          variant="secondary"
          size="small"
          disabled={loading || saving}
          onclick={onReload}
        >
          Reload
        </UiButton>
        <UiButton
          size="small"
          disabled={loading || jobsLoading || syncAllDisabled}
          onclick={onSyncAll}
        >
          Sync All
        </UiButton>
      </div>
    </div>

    <div class="account-summary-strip" aria-label="Account summary">
      <div class="account-stat">
        <span>{accounts.length}</span>
        <small>Total</small>
      </div>
      <div class="account-stat">
        <span>{enabledAccountCount(accounts)}</span>
        <small>Enabled</small>
      </div>
      <div class="account-stat">
        <span>{credentialedAccountCount(accounts)}</span>
        <small>Credentials</small>
      </div>
      <div class="account-stat">
        <span>{syncingCount}</span>
        <small>Syncing</small>
      </div>
    </div>

    <div class="account-list">
      {#if loading}
        <div class="empty-state">Loading</div>
      {:else if accounts.length === 0}
        <div class="empty-state">No accounts</div>
      {:else}
        {#each accounts as account (account.id)}
          <AccountSourceRow
            {account}
            selected={editingAccountId === account.id}
            statusLabel={getStatusLabel(account)}
            statusTone={getStatusTone(account)}
            activeSyncJob={getActiveSyncJob(account.id)}
            {onToggleEnabled}
            onSelect={onEdit}
            {onSync}
            {onCancelSync}
            {onRemove}
          />
        {/each}
      {/if}
    </div>
  </section>

  <AccountEditor
    editing={Boolean(editingAccountId)}
    {saving}
    bind:label
    bind:loginName
    bind:password
    {onReset}
    {onSave}
  />
</div>

<style>
  .accounts-layout {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(320px, 420px);
    gap: 18px;
    align-items: start;
    min-height: 0;
    overflow: auto;
  }

  .accounts-panel {
    min-width: 0;
    padding: 18px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 16px 40px rgb(0 0 0 / 18%);
  }

  .panel-title,
  .panel-actions {
    display: flex;
    align-items: center;
  }

  .panel-title {
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 14px;
  }

  .panel-title > div {
    min-width: 0;
  }

  .panel-actions {
    gap: 8px;
  }

  h2 {
    margin: 0;
    color: var(--text-strong);
    font-size: 17px;
    font-weight: 700;
  }

  .panel-title p {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.35;
  }

  .account-summary-strip {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 1px;
    margin-bottom: 14px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--border);
    overflow: hidden;
  }

  .account-stat {
    display: grid;
    gap: 2px;
    padding: 10px 12px;
    background: var(--panel-soft);
  }

  .account-stat span {
    color: var(--text-strong);
    font-size: 18px;
    font-weight: 700;
    line-height: 1;
  }

  .account-stat small {
    color: var(--muted);
    font-size: 12px;
  }

  .account-list {
    display: grid;
    gap: 8px;
  }

  .empty-state {
    padding: 16px 8px;
    color: var(--muted);
    text-align: center;
  }

  @media (max-width: 980px) {
    .accounts-layout {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 720px) {
    .panel-title,
    .panel-actions {
      align-items: stretch;
      flex-direction: column;
    }

    .account-summary-strip {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
