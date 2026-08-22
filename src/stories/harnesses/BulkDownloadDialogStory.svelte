<script lang="ts">
  import BulkDownloadDialogView from "$lib/components/BulkDownloadDialog.svelte";
  import type { BulkDownloadDialog } from "$lib/model/types";

  let {
    kind = "confirm",
    failedCount = 0,
  }: {
    kind?: BulkDownloadDialog["kind"];
    failedCount?: number;
  } = $props();

  let lastAction = $state("Dialog open");

  const preview = $derived({
    totalCount: 38,
    requestedCount: kind === "notice" ? 0 : 24,
    skippedDownloadedCount: kind === "notice" ? 31 : 9,
    skippedQueuedCount: kind === "notice" ? 7 : 5,
    plannedCount: kind === "notice" ? 0 : 22,
    failedCount,
    knownExpectedBytes: kind === "notice" ? 0 : 6_420_000_000,
    totalExpectedBytes: kind === "notice" ? 0 : null,
    unknownSizeCount: kind === "notice" ? 0 : 3,
  });
</script>

<BulkDownloadDialogView
  dialog={{ kind, preview }}
  onClose={(confirmed) => (lastAction = confirmed ? "Start download" : "Close dialog")}
/>

<p aria-live="polite">{lastAction}</p>

<style>
  p {
    position: fixed;
    z-index: 60;
    right: 14px;
    bottom: 10px;
    margin: 0;
    color: var(--text-subtle);
    font-size: 12px;
  }
</style>
