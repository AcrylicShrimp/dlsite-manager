<script lang="ts">
  import DownloadsView from "$lib/features/downloads/DownloadsView.svelte";
  import type { JobSnapshot } from "$lib/model/types";
  import {
    cancellingDownloadJob,
    queuedDownloadJob,
    runningDownloadJob,
  } from "../fixtures/jobs";

  let {
    viewState = "active",
  }: {
    viewState?: "active" | "loading" | "empty";
  } = $props();

  const titles: Record<string, string> = {
    "job-queued": "A Long Evening at the Observatory — Binaural Voice Drama",
    "job-running": "Imported local comic with partial metadata",
    "job-cancelling": "Bulk download (30 works)",
  };
  const details: Record<string, string> = {
    "job-queued": "Waiting to start",
    "job-running": "307.1 MB of 459.7 MB",
    "job-cancelling": "12 of 30 products processed",
  };

  let lastAction = $state("No action yet");

  const jobs = $derived(
    viewState === "active"
      ? [queuedDownloadJob, runningDownloadJob, cancellingDownloadJob]
      : [],
  );

  function cancel(job: JobSnapshot) {
    lastAction = `Cancel ${job.id}`;
  }
</script>

<main class="story-surface">
  <DownloadsView
    {jobs}
    loading={viewState === "loading"}
    queuedCount={jobs.filter((job) => job.status === "queued").length}
    runningCount={jobs.filter((job) => job.status === "running").length}
    getTitle={(job) => titles[job.id] ?? job.title}
    getDetail={(job) => details[job.id] ?? "Download status unavailable"}
    onReload={() => (lastAction = "Reload")}
    onCancel={cancel}
  />
  <p aria-live="polite">{lastAction}</p>
</main>

<style>
  .story-surface {
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    gap: 12px;
    width: min(1120px, calc(100vw - 48px));
    min-height: min(680px, calc(100vh - 48px));
    margin: 0 auto;
    padding: 24px 0;
  }

  p {
    margin: 0;
    color: var(--text-subtle);
    font-size: 12px;
    text-align: right;
  }

  @media (max-width: 720px) {
    .story-surface {
      width: calc(100vw - 24px);
      min-height: calc(100vh - 24px);
      padding: 12px 0;
    }
  }
</style>
