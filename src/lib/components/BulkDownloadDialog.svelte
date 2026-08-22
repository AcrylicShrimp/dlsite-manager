<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import type { BulkDownloadDialog } from "$lib/model/types";
  import { bulkDownloadExpectedBytesLabel } from "$lib/utils/format";

  let {
    dialog,
    onClose,
  }: {
    dialog: BulkDownloadDialog | null;
    onClose?: (confirmed: boolean) => void;
  } = $props();
</script>

{#if dialog}
  <div
    class="dialog-layer"
    role={dialog.kind === "notice" ? "alertdialog" : "dialog"}
    aria-modal="true"
    aria-labelledby="bulk-dialog-title"
    onkeydown={(event) => {
      if (event.key === "Escape") onClose?.(false);
    }}
  >
    <button
      class="backdrop"
      type="button"
      aria-label="Close bulk download dialog"
      onclick={() => onClose?.(false)}
    ></button>
    <section class="panel">
      <div class="heading">
        <div>
          <p>Bulk Download</p>
          <h2 id="bulk-dialog-title">
            {dialog.kind === "notice" ? "No products to download" : "Start bulk download?"}
          </h2>
        </div>
        <button
          class="close"
          type="button"
          aria-label="Close bulk download dialog"
          onclick={() => onClose?.(false)}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M18 6 6 18M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div class="summary" aria-label="Bulk download plan">
        <div>
          <span>Products to download</span>
          <strong>{dialog.preview.requestedCount}</strong>
        </div>
        <div>
          <span>Checked products</span>
          <strong>{dialog.preview.plannedCount}</strong>
        </div>
        <div>
          <span>Already downloaded</span>
          <strong>{dialog.preview.skippedDownloadedCount}</strong>
        </div>
        <div>
          <span>Already queued</span>
          <strong>{dialog.preview.skippedQueuedCount}</strong>
        </div>
        <div class="wide">
          <span>Expected total download</span>
          <strong>{bulkDownloadExpectedBytesLabel(dialog.preview)}</strong>
        </div>
      </div>

      {#if dialog.preview.failedCount > 0}
        <p class="warning">
          {dialog.preview.failedCount} product(s) could not be checked before download. They will still be attempted and may fail.
        </p>
      {/if}

      {#if dialog.kind === "notice"}
        <p class="note">
          Matching products were already downloaded, already queued, or unavailable for this action.
        </p>
      {/if}

      <div class="actions" class:notice={dialog.kind === "notice"}>
        {#if dialog.kind === "notice"}
          <UiButton onclick={() => onClose?.(false)}>Close</UiButton>
        {:else}
          <UiButton variant="secondary" onclick={() => onClose?.(false)}>Cancel</UiButton>
          <UiButton onclick={() => onClose?.(true)}>Start Download</UiButton>
        {/if}
      </div>
    </section>
  </div>
{/if}

<style>
  .dialog-layer {
    position: fixed;
    z-index: 50;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 24px;
  }

  .backdrop {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    min-width: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: rgb(0 0 0 / 68%);
    cursor: default;
  }

  .panel {
    position: relative;
    z-index: 1;
    display: grid;
    gap: 16px;
    width: min(560px, calc(100vw - 40px));
    max-height: calc(100vh - 48px);
    padding: 18px;
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 24px 64px rgb(0 0 0 / 52%);
    overflow: auto;
  }

  .heading {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 12px;
    align-items: start;
  }

  .heading p,
  .note,
  .warning {
    margin: 0;
  }

  .heading p {
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
  }

  h2 {
    margin: 2px 0 0;
    color: var(--text-strong);
    font-size: 20px;
  }

  .close {
    width: 34px;
    min-width: 34px;
    height: 34px;
    padding: 0;
    border-color: var(--border-strong);
    color: var(--muted);
    background: var(--panel-raised);
  }

  .close:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .close:focus-visible {
    outline: none;
    box-shadow: var(--focus-ring);
  }

  .close svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2.35;
  }

  .summary {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--border);
    overflow: hidden;
  }

  .summary div {
    display: grid;
    gap: 4px;
    padding: 11px 12px;
    background: var(--panel-soft);
  }

  .summary .wide {
    grid-column: 1 / -1;
  }

  .summary span {
    color: var(--muted);
    font-size: 12px;
    font-weight: 650;
  }

  .summary strong {
    min-width: 0;
    color: var(--text-strong);
    font-size: 17px;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }

  .warning {
    padding: 10px 12px;
    border: 1px solid rgb(248 113 113 / 36%);
    border-radius: 8px;
    color: #fecaca;
    background: rgb(248 113 113 / 11%);
    font-size: 13px;
    line-height: 1.45;
  }

  .note {
    color: var(--muted);
    font-size: 13px;
    line-height: 1.45;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  @media (max-width: 720px) {
    .dialog-layer {
      padding: 12px;
    }

    .panel {
      width: 100%;
      max-height: calc(100vh - 24px);
      padding: 14px;
    }

    .actions {
      display: grid;
      grid-template-columns: 1fr 1fr;
    }

    .actions.notice {
      grid-template-columns: 1fr;
    }
  }
</style>
