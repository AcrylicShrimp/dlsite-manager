<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import type { JobSnapshot } from "$lib/model/types";
  import DownloadQueueRow from "./DownloadQueueRow.svelte";

  let {
    jobs = [],
    loading = false,
    queuedCount = 0,
    runningCount = 0,
    getTitle,
    getDetail,
    onReload,
    onCancel,
  }: {
    jobs?: JobSnapshot[];
    loading?: boolean;
    queuedCount?: number;
    runningCount?: number;
    getTitle: (job: JobSnapshot) => string;
    getDetail: (job: JobSnapshot) => string;
    onReload: () => void;
    onCancel: (job: JobSnapshot) => void;
  } = $props();
</script>

<section class="downloads-panel" aria-label="Downloads">
  <div class="panel-title">
    <div>
      <h2>Download queue</h2>
      <p>Currently queued and running downloads</p>
    </div>
    <UiButton
      variant="secondary"
      size="small"
      responsiveWidth="auto"
      disabled={loading}
      onclick={onReload}
    >
      Reload
    </UiButton>
  </div>

  <div class="download-summary-strip" aria-label="Download queue summary">
    <div class="download-stat">
      <span>{jobs.length}</span>
      <small>Current</small>
    </div>
    <div class="download-stat">
      <span>{queuedCount}</span>
      <small>Queued</small>
    </div>
    <div class="download-stat">
      <span>{runningCount}</span>
      <small>Running</small>
    </div>
  </div>

  {#if loading}
    <div class="empty-state">Loading</div>
  {:else if jobs.length === 0}
    <div class="empty-state">No active downloads</div>
  {:else}
    <div class="download-queue-list" aria-label="Download jobs">
      {#each jobs as job (job.id)}
        <DownloadQueueRow
          {job}
          title={getTitle(job)}
          detail={getDetail(job)}
          {onCancel}
        />
      {/each}
    </div>
  {/if}
</section>

<style>
  .downloads-panel {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    gap: 14px;
    min-width: 0;
    min-height: 0;
    padding: 18px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 16px 40px rgb(0 0 0 / 18%);
    overflow: hidden;
  }

  .panel-title {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .panel-title > div {
    min-width: 0;
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

  .download-summary-strip {
    display: grid;
    flex: 0 0 auto;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 1px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--border);
    overflow: hidden;
  }

  .download-stat {
    display: grid;
    gap: 2px;
    padding: 10px 12px;
    background: var(--panel-soft);
  }

  .download-stat span {
    color: var(--text-strong);
    font-size: 18px;
    font-weight: 700;
    line-height: 1.1;
  }

  .download-stat small {
    color: var(--muted);
    font-size: 12px;
    font-weight: 650;
  }

  .download-queue-list {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    min-height: 0;
    padding-right: 4px;
    overflow: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
  }

  .empty-state {
    padding: 36px 14px;
    color: var(--muted);
    text-align: center;
  }

  @media (max-width: 720px) {
    .panel-title {
      align-items: stretch;
      flex-direction: column;
    }

    .download-summary-strip {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
