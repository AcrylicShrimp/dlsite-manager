<script lang="ts">
  import JobDetails from "$lib/features/activity/JobDetails.svelte";
  import type { JobSnapshot } from "$lib/model/types";
  import {
    downloadQueueProgressPercent,
    jobLabel,
    isActiveJob,
  } from "$lib/utils/jobs";
  import Button from "$lib/components/workspace/Button.svelte";
  import Progress from "$lib/components/workspace/Progress.svelte";
  import Modal from "$lib/components/workspace/Modal.svelte";
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
  let expanded = $state(false);
  const percent = $derived(downloadQueueProgressPercent(job));
  const disabled = $derived(
    !job.cancellable || job.status === "cancelling" || !isActiveJob(job),
  );
</script>

<article
  class="download-queue-row dm:border-0 dm:border-b dm:border-solid dm:border-draft-line dm:px-4 dm:py-4"
>
  <p class="dm:mt-0 dm:mb-2 dm:text-xs dm:text-draft-dim">{jobLabel(job)}</p>
  <div class="dm:mb-3 dm:flex dm:items-start dm:justify-between dm:gap-3">
    <button
      type="button"
      onclick={() => (expanded = true)}
      class="dm:min-w-0 dm:wrap-anywhere dm:border-0 dm:bg-transparent dm:p-0 dm:text-left dm:font-[inherit] dm:font-semibold dm:text-draft-ink dm:draft-focus-text"
      >{title}</button
    >
    <Button variant="text" tone="muted" {disabled} onclick={() => onCancel(job)}
      >Cancel</Button
    >
  </div>
  {#if job.status !== "queued"}<Progress
      value={percent}
      label={`${title} progress`}
    />
    <div
      class="dm:mt-2 dm:flex dm:justify-between dm:gap-3 dm:text-xs dm:text-draft-dim"
    >
      <span class="dm:wrap-anywhere">{detail}</span>{#if percent !== null}<span
          >{percent}%</span
        >{/if}
    </div>{/if}
</article>
{#if expanded}<Modal
    title="Download details"
    onClose={() => (expanded = false)}
  >
    <JobDetails {job} {title} {detail} />
    {#snippet actions()}<Button {disabled} onclick={() => onCancel(job)}
        >Cancel download</Button
      >{/snippet}
  </Modal>{/if}
