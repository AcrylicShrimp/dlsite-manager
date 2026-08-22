<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import type { JobSnapshot } from "$lib/model/types";
  import {
    downloadQueueKindLabel,
    downloadQueueProgressPercent,
    downloadQueueSubtitle,
    downloadQueueTime,
    isActiveJob,
    jobLabel,
  } from "$lib/utils/jobs";

  let {
    job,
    title,
    detail,
    onCancel,
  }: {
    job: JobSnapshot;
    title: string;
    detail: string;
    onCancel: (job: JobSnapshot) => void;
  } = $props();

  const progressPercent = $derived(downloadQueueProgressPercent(job));
  const active = $derived(isActiveJob(job));
</script>

<article class="download-queue-row" class:failed={job.status === "failed"} data-status={job.status}>
  <div class="download-queue-main">
    <span>{downloadQueueKindLabel(job)}</span>
    <h2 title={title}>{title}</h2>
    <p title={downloadQueueSubtitle(job)}>{downloadQueueSubtitle(job)}</p>
  </div>

  <div class="download-queue-state">
    <div>
      <strong class:active>{jobLabel(job)}</strong>
      <small>{detail}</small>
    </div>
    {#if progressPercent !== null}
      <div class="download-progress-track" aria-label={`Progress ${progressPercent}%`}>
        <span style={`width: ${progressPercent}%`}></span>
      </div>
    {/if}
  </div>

  <time datetime={job.finishedAt ?? job.startedAt ?? job.createdAt}>
    {downloadQueueTime(job)}
  </time>

  {#if active}
    <div class="download-queue-action">
      <UiButton
        variant="secondary"
        size="small"
        responsiveWidth="auto"
        disabled={!job.cancellable || job.status === "cancelling"}
        onclick={() => onCancel(job)}
      >
        Cancel
      </UiButton>
    </div>
  {/if}
</article>

<style>
  .download-queue-row {
    display: grid;
    flex: 0 0 auto;
    grid-template-columns: minmax(220px, 1.2fr) minmax(220px, 0.8fr) minmax(150px, auto) auto;
    gap: 14px;
    align-items: center;
    min-height: 76px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
  }

  .download-queue-row:last-child {
    border-bottom: 0;
  }

  .download-queue-row.failed h2 {
    color: var(--danger);
  }

  .download-queue-main,
  .download-queue-state {
    min-width: 0;
  }

  .download-queue-main span {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 8px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    color: var(--muted);
    background: var(--panel-soft);
    font-size: 11px;
    font-weight: 700;
    line-height: 1;
  }

  .download-queue-main h2 {
    margin: 7px 0 0;
    color: var(--text-strong);
    font-size: 15px;
    line-height: 1.25;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .download-queue-main p,
  .download-queue-state small,
  time {
    color: var(--muted);
    font-size: 12px;
  }

  .download-queue-main p {
    margin: 3px 0 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .download-queue-state {
    display: grid;
    gap: 8px;
  }

  .download-queue-state > div:first-child {
    display: grid;
    gap: 2px;
  }

  .download-queue-state strong {
    color: var(--text);
    font-size: 13px;
    font-weight: 700;
  }

  .download-queue-state strong.active {
    color: var(--accent);
  }

  .download-progress-track {
    height: 6px;
    border-radius: 999px;
    background: var(--field);
    overflow: hidden;
  }

  .download-progress-track span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--accent);
  }

  time {
    text-align: right;
    white-space: nowrap;
  }

  @media (max-width: 1220px) {
    .download-queue-row {
      grid-template-columns: minmax(0, 1fr) minmax(180px, 0.7fr) auto;
    }

    .download-queue-action {
      grid-column: 1 / -1;
      justify-self: start;
    }
  }

  @media (max-width: 720px) {
    .download-queue-row {
      grid-template-columns: 1fr;
    }

    time {
      text-align: left;
    }
  }
</style>
