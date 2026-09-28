<script lang="ts">
  import type { JobSnapshot } from "$lib/model/types";
  import { progress, jobDetail } from "../fixtures/redesign";
  import Button from "./DraftButton.svelte";
  import Progress from "./DraftProgress.svelte";
  let {
    job,
    onDetails,
    onCancel,
  }: { job: JobSnapshot; onDetails: () => void; onCancel: () => void } =
    $props();
</script>

<article
  class="download-row dm:border-0 dm:border-b dm:border-solid dm:border-draft-line dm:px-4 dm:py-4"
>
  <div
    class="dm:mb-2 dm:flex dm:items-center dm:gap-2 dm:text-xs dm:text-draft-dim"
  >
    <span
      class={`status ${job.status === "running" ? "dm:text-draft-accent" : ""}`}
      >{job.status === "running"
        ? job.phase === "unpacking"
          ? "Unpacking"
          : "Downloading"
        : job.status === "cancelling"
          ? "Cancelling"
          : "Queued"}</span
    ><small class="dm:text-xs">{String(job.metadata.workId ?? "")}</small>
  </div>
  <div class="dm:mb-3 dm:flex dm:items-start dm:justify-between dm:gap-3">
    <button
      type="button"
      class="row-title dm:min-w-0 dm:border-0 dm:bg-transparent dm:p-0 dm:font-[inherit] dm:text-base dm:font-semibold dm:leading-normal dm:text-left dm:wrap-anywhere dm:text-draft-ink dm:cursor-pointer dm:hover:text-draft-accent dm:hover:underline dm:draft-focus-text"
      onclick={onDetails}>{job.title}</button
    >
    <Button variant="text" tone="muted" onclick={onCancel}>Cancel</Button>
  </div>
  {#if job.status === "running"}<Progress
      value={progress(job)}
      label={`${job.title} progress`}
    />
    <div
      class="dm:mt-2 dm:flex dm:justify-between dm:gap-3 dm:text-xs dm:text-draft-dim"
    >
      <span>{jobDetail(job)}</span><strong
        class="dm:font-medium dm:text-draft-accent">{progress(job)}%</strong
      >
    </div>{/if}
</article>
