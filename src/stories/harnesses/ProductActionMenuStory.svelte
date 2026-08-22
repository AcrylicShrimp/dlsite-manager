<script lang="ts">
  import ProductActionMenu from "$lib/features/library/ProductActionMenu.svelte";
  import type { WorkDownloadStatus } from "$lib/model/types";

  let {
    downloadStatus = "notDownloaded",
    busy = false,
  }: {
    downloadStatus?: WorkDownloadStatus;
    busy?: boolean;
  } = $props();

  let lastAction = $state("No action yet");
</script>

<main class="story-surface">
  <div class="anchor">
    <span>RJ01553954</span>
    <ProductActionMenu
      workId="RJ01553954"
      {downloadStatus}
      {busy}
      onClose={() => (lastAction = "Close menu")}
      onDownloadArchives={() => (lastAction = "Download archives only")}
      onMarkDownloaded={() => (lastAction = "Mark as downloaded")}
      onRedownload={() => (lastAction = "Re-download")}
      onDeleteDownload={() => (lastAction = "Delete download")}
    />
  </div>
  <p aria-live="polite">{lastAction}</p>
</main>

<style>
  .story-surface {
    display: grid;
    place-content: center;
    min-height: 420px;
    padding: 24px;
  }

  .anchor {
    display: grid;
    justify-items: end;
    gap: 8px;
  }

  .anchor > span {
    color: var(--text-subtle);
    font-size: 12px;
    font-weight: 700;
  }

  p {
    margin: 12px 0 0;
    color: var(--text-subtle);
    font-size: 12px;
    text-align: right;
  }
</style>
