<script lang="ts">
  import DownloadQueueRow from "$lib/features/downloads/DownloadQueueRow.svelte";
  import type { JobSnapshot } from "$lib/model/types";

  let { job }: { job: JobSnapshot } = $props();
  let lastAction = $state("No action yet");
</script>

<main class="story-surface">
  <DownloadQueueRow
    {job}
    title="Work download that could not be completed"
    detail={job.error?.message ?? "Download status unavailable"}
    onCancel={(selectedJob) => (lastAction = `Cancel ${selectedJob.id}`)}
  />
  <p aria-live="polite">{lastAction}</p>
</main>

<style>
  .story-surface {
    display: grid;
    gap: 12px;
    width: min(1120px, calc(100vw - 48px));
    margin: 24px auto;
    padding: 0 18px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
  }

  p {
    margin: 0 0 12px;
    color: var(--text-subtle);
    font-size: 12px;
    text-align: right;
  }

  @media (max-width: 720px) {
    .story-surface {
      width: calc(100vw - 24px);
      margin: 12px auto;
    }
  }
</style>
