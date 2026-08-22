<script lang="ts">
  import type { WorkDownloadStatus } from "$lib/model/types";

  let {
    workId,
    downloadStatus,
    busy = false,
    left,
    top,
    onClose,
    onDownloadArchives,
    onMarkDownloaded,
    onRedownload,
    onDeleteDownload,
  }: {
    workId: string;
    downloadStatus: WorkDownloadStatus;
    busy?: boolean;
    left?: number;
    top?: number;
    onClose?: () => void;
    onDownloadArchives?: () => void;
    onMarkDownloaded?: () => void;
    onRedownload?: () => void;
    onDeleteDownload?: () => void;
  } = $props();

  const positioned = $derived(left !== undefined && top !== undefined);
</script>

<div
  class="action-menu"
  class:positioned
  role="menu"
  tabindex="-1"
  aria-label={`Actions for ${workId}`}
  style={positioned ? `left: ${left}px; top: ${top}px;` : undefined}
  onclick={(event) => event.stopPropagation()}
  onkeydown={(event) => {
    if (event.key === "Escape") onClose?.();
  }}
>
  {#if downloadStatus !== "downloaded"}
    <button type="button" role="menuitem" disabled={busy} onclick={onDownloadArchives}>
      Download Archives Only
    </button>
    <button type="button" role="menuitem" disabled={busy} onclick={onMarkDownloaded}>
      Mark as Downloaded
    </button>
  {/if}
  {#if downloadStatus === "downloaded"}
    <button class="danger" type="button" role="menuitem" disabled={busy} onclick={onRedownload}>
      Re-download
    </button>
  {/if}
  {#if downloadStatus !== "notDownloaded"}
    <button
      class="danger"
      type="button"
      role="menuitem"
      disabled={busy}
      onclick={onDeleteDownload}
    >
      Delete Download
    </button>
  {/if}
</div>

<style>
  .action-menu {
    display: grid;
    width: 220px;
    gap: 4px;
    padding: 6px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel-raised);
    box-shadow: 0 18px 40px rgb(0 0 0 / 38%);
  }

  .action-menu.positioned {
    position: fixed;
    z-index: 80;
  }

  button {
    justify-content: flex-start;
    width: 100%;
    min-height: 34px;
    padding: 0 10px;
    border: 0;
    color: var(--text);
    background: transparent;
    font: inherit;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    background: var(--panel-soft);
  }

  button:focus-visible {
    outline: none;
    background: var(--panel-soft);
    box-shadow: inset var(--focus-ring);
  }

  button.danger {
    color: var(--danger);
  }

  button.danger:hover:not(:disabled),
  button.danger:focus-visible {
    background: rgb(248 113 113 / 11%);
  }

  button:disabled {
    cursor: default;
    opacity: 0.58;
  }
</style>
