<script lang="ts">
  import Modal from "$lib/components/workspace/Modal.svelte";
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
  <Modal
    title={dialog.kind === "notice"
      ? "No products to download"
      : "Start bulk download?"}
    onClose={() => onClose?.(false)}
  >
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
        {dialog.preview.failedCount} product(s) could not be checked before download.
        They will still be attempted and may fail.
      </p>
    {/if}

    {#if dialog.kind === "notice"}
      <p class="note">
        Matching products were already downloaded, already queued, or
        unavailable for this action.
      </p>
    {/if}

    <div class="actions" class:notice={dialog.kind === "notice"}>
      {#if dialog.kind === "notice"}
        <UiButton onclick={() => onClose?.(false)}>Close</UiButton>
      {:else}
        <UiButton variant="secondary" onclick={() => onClose?.(false)}
          >Cancel</UiButton
        >
        <UiButton onclick={() => onClose?.(true)}>Start Download</UiButton>
      {/if}
    </div>
  </Modal>
{/if}

<style>
  .summary {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }
  .summary div {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .summary span {
    color: var(--muted);
    font-size: 12px;
  }
  .wide {
    grid-column: 1/-1;
  }
  .warning {
    color: var(--danger);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 24px;
  }
</style>
