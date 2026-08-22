<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import type { Account, JobSnapshot } from "$lib/model/types";
  import {
    accountCredentialLabel,
    accountEnabledLabel,
    accountLastSyncLabel,
    accountLoginLabel,
  } from "$lib/utils/accounts";

  type AccountStatusTone = "synced" | "syncing" | "failed" | "warning" | "disabled" | "idle";

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

<article class="account-row" class:disabled={!account.enabled} class:selected>
  <button
    class="account-enabled-pill"
    class:disabled={!account.enabled}
    type="button"
    title={account.enabled ? "Disable account" : "Enable account"}
    aria-label={account.enabled ? `Disable ${account.label}` : `Enable ${account.label}`}
    onclick={() => onToggleEnabled(account, !account.enabled)}
    disabled={Boolean(activeSyncJob)}
  >
    {accountEnabledLabel(account)}
  </button>

  <button class="account-name" type="button" onclick={() => onSelect(account)}>
    <span class="account-identity">
      <span title={account.label}>{account.label}</span>
      <small title={accountLoginLabel(account)}>{accountLoginLabel(account)}</small>
    </span>
  </button>

  <div class="account-meta-grid">
    <div>
      <span>Status</span>
      <strong class={`account-status-text ${statusTone}`} title={statusLabel}>{statusLabel}</strong>
    </div>
    <div>
      <span>Credential</span>
      <strong title={accountCredentialLabel(account)}>{accountCredentialLabel(account)}</strong>
    </div>
    <div>
      <span>Last sync</span>
      <strong title={accountLastSyncLabel(account)}>{accountLastSyncLabel(account)}</strong>
    </div>
  </div>

  <div class="account-actions">
    {#if activeSyncJob}
      <UiButton
        variant="secondary"
        size="small"
        responsiveWidth="auto"
        disabled={!activeSyncJob.cancellable || activeSyncJob.status === "cancelling"}
        onclick={() => onCancelSync(account)}
      >
        Cancel
      </UiButton>
    {:else}
      <UiButton
        size="small"
        responsiveWidth="auto"
        disabled={!account.enabled}
        onclick={() => onSync(account)}
      >
        Sync
      </UiButton>
    {/if}
    <UiButton
      variant="secondary"
      size="small"
      responsiveWidth="auto"
      title="Update saved credential"
      onclick={() => onSelect(account)}
    >
      Update Credential
    </UiButton>
    <UiButton
      variant="secondary"
      size="small"
      responsiveWidth="auto"
      title="Remove account source"
      disabled={Boolean(activeSyncJob)}
      onclick={() => onRemove(account)}
    >
      Remove
    </UiButton>
  </div>
</article>

<style>
  .account-row {
    display: grid;
    grid-template-columns: minmax(220px, 1fr) minmax(340px, 0.88fr);
    grid-template-rows: auto auto auto;
    gap: 10px 30px;
    align-items: start;
    min-height: 132px;
    padding: 14px 16px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-soft);
  }

  .account-row.selected {
    border-color: var(--accent);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .account-row.disabled {
    opacity: 0.62;
  }

  .account-name {
    display: grid;
    grid-column: 1;
    grid-row: 2 / 4;
    align-self: start;
    justify-content: stretch;
    justify-items: stretch;
    width: 100%;
    min-width: 0;
    height: auto;
    min-height: 0;
    padding: 0;
    border: 0;
    color: inherit;
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }

  .account-status-text.synced {
    color: var(--accent);
  }

  .account-status-text.syncing,
  .account-status-text.warning {
    color: #d8a62d;
  }

  .account-status-text.failed {
    color: var(--danger);
  }

  .account-identity {
    display: grid;
    gap: 3px;
    justify-self: start;
    width: min(360px, 100%);
    min-width: 0;
  }

  .account-identity span {
    max-width: 100%;
    color: var(--text);
    font-size: 15px;
    font-weight: 650;
    line-height: 1.1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .account-identity small {
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .account-enabled-pill {
    display: inline-flex;
    align-items: center;
    grid-column: 1;
    grid-row: 1;
    justify-content: flex-start;
    justify-self: start;
    gap: 6px;
    min-width: 114px;
    min-height: 28px;
    padding: 3px 10px;
    border: 1px solid rgb(112 165 120 / 58%);
    border-radius: 5px;
    color: var(--accent);
    background: var(--accent-muted);
    font: inherit;
    font-size: 12px;
    font-weight: 650;
    line-height: 1.1;
    cursor: pointer;
  }

  .account-enabled-pill::before {
    content: "";
    width: 9px;
    height: 9px;
    border: 2px solid currentColor;
    border-radius: 999px;
    background: rgb(160 198 164 / 24%);
  }

  .account-enabled-pill.disabled {
    border-color: var(--border-strong);
    color: var(--text-subtle);
    background: var(--field-disabled);
  }

  .account-enabled-pill:disabled {
    cursor: default;
  }

  .account-enabled-pill:focus-visible,
  .account-name:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .account-meta-grid {
    display: grid;
    grid-column: 2;
    grid-row: 2;
    align-self: start;
    grid-template-columns: 1fr;
    gap: 7px;
    min-width: 0;
  }

  .account-meta-grid div {
    display: grid;
    grid-template-columns: minmax(92px, 0.42fr) minmax(170px, 1fr);
    gap: 16px;
    align-items: baseline;
    min-width: 0;
  }

  .account-meta-grid span {
    color: var(--muted);
    font-size: 13px;
    font-weight: 600;
  }

  .account-meta-grid strong {
    min-width: 0;
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .account-actions {
    display: flex;
    grid-column: 2;
    grid-row: 3;
    align-self: end;
    justify-content: flex-end;
    gap: 8px;
    flex-wrap: wrap;
  }

  @media (max-width: 1220px) {
    .account-row {
      grid-template-columns: minmax(0, 1fr);
    }

    .account-name {
      grid-column: 1;
      grid-row: 2;
    }

    .account-meta-grid {
      grid-column: 1 / -1;
      grid-row: 3;
    }

    .account-actions {
      grid-column: 1 / -1;
      grid-row: 4;
      justify-content: flex-start;
    }
  }

  @media (max-width: 720px) {
    .account-row,
    .account-meta-grid {
      grid-template-columns: 1fr;
    }

    .account-row {
      align-items: stretch;
    }

    .account-enabled-pill,
    .account-name,
    .account-meta-grid,
    .account-actions {
      grid-column: 1;
      grid-row: auto;
      justify-content: flex-start;
    }
  }
</style>
