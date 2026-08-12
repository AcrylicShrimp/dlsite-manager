<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import type { JobSnapshot } from "$lib/model/types";
  import { isActiveJob, jobLabel } from "$lib/utils/jobs";

  let {
    jobs = [],
    loading = false,
    getTitle,
    getDetail,
    onCancel,
  }: {
    jobs?: JobSnapshot[];
    loading?: boolean;
    getTitle: (job: JobSnapshot) => string;
    getDetail: (job: JobSnapshot) => string;
    onCancel: (job: JobSnapshot) => void;
  } = $props();
</script>

{#if loading}
  <div class="empty-state">Loading</div>
{:else if jobs.length === 0}
  <div class="empty-state">No jobs</div>
{:else}
  <div class="job-list" aria-label="Recent jobs">
    {#each jobs as job (job.id)}
      <article class="job-row" class:failed={job.status === "failed"} data-status={job.status}>
        <div>
          <div class="job-title">{getTitle(job)}</div>
          <div class="job-detail">{getDetail(job)}</div>
        </div>
        <span class:active={isActiveJob(job)}>{jobLabel(job)}</span>
        {#if isActiveJob(job)}
          <UiButton
            variant="secondary"
            size="small"
            responsiveWidth="auto"
            disabled={!job.cancellable || job.status === "cancelling"}
            onclick={() => onCancel(job)}
          >
            Cancel
          </UiButton>
        {/if}
      </article>
    {/each}
  </div>
{/if}

<style>
  .job-list {
    display: grid;
    flex: 1 1 auto;
    gap: 0;
    align-content: start;
    grid-auto-rows: max-content;
    min-height: 0;
    padding-right: 4px;
    overflow: auto;
    overscroll-behavior: contain;
    scrollbar-gutter: stable;
  }

  .job-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    gap: 8px;
    align-items: center;
    min-height: 56px;
    padding: 10px 0;
    border-bottom: 1px solid var(--border);
  }

  .job-row:last-child {
    border-bottom: 0;
  }

  .job-row > div {
    min-width: 0;
  }

  .job-row.failed .job-title {
    color: var(--danger);
  }

  .job-title {
    color: var(--text);
    font-size: 13px;
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .job-detail {
    margin-top: 2px;
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .job-row > span {
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }

  .job-row > span.active {
    color: var(--accent);
    font-weight: 650;
  }

  .empty-state {
    padding: 36px 14px;
    color: var(--muted);
    text-align: center;
  }

  @media (max-width: 720px) {
    .job-row {
      grid-template-columns: 1fr;
    }
  }
</style>
