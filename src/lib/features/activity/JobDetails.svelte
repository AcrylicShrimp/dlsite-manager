<script lang="ts">
  import type { JobSnapshot } from "$lib/model/types";
  import { bulkDownloadResult, jobLabel } from "$lib/utils/jobs";
  import { detailDate } from "$lib/utils/format";
  import Disclosure from "$lib/components/workspace/Disclosure.svelte";
  let {
    job,
    title,
    detail,
  }: { job: JobSnapshot; title: string; detail: string } = $props();
  const bulk = $derived(bulkDownloadResult(job));
</script>

<h3 class="dm:mt-0 dm:wrap-anywhere">{title}</h3>
<p>{jobLabel(job)}</p>
<p class="dm:wrap-anywhere">{detail}</p>
{#if job.error}<p class="dm:wrap-anywhere dm:text-draft-error">
    {job.error.code} · {job.error.message}
  </p>{/if}
{#if bulk.failedWorks.length}<Disclosure
    title={`Failed works (${bulk.failedWorks.length})`}
    open
  >
    <ul class="dm:space-y-3 dm:pl-4">
      {#each bulk.failedWorks as work}<li class="dm:wrap-anywhere">
          <strong>{work.workId}</strong>
          <p class="dm:m-0 dm:text-draft-error">
            {work.errorMessage ?? work.errorCode ?? "Failed"}
          </p>
        </li>{/each}
    </ul>
  </Disclosure>{/if}
{#if bulk.succeededWorks.length}<Disclosure
    title={`Downloaded works (${bulk.succeededWorks.length})`}
  >
    <ul class="dm:space-y-3 dm:pl-4">
      {#each bulk.succeededWorks as work}<li class="dm:wrap-anywhere">
          <strong>{work.workId}</strong>
          <p class="dm:m-0 dm:text-draft-dim">{work.localPath ?? "—"}</p>
        </li>{/each}
    </ul>
  </Disclosure>{/if}
<Disclosure title="Job details"
  ><dl class="dm:grid dm:grid-cols-[auto_minmax(0,1fr)] dm:gap-3 dm:text-xs">
    <dt>Created</dt>
    <dd class="dm:m-0">{detailDate(job.createdAt)}</dd>
    <dt>Started</dt>
    <dd class="dm:m-0">{detailDate(job.startedAt)}</dd>
    <dt>Finished</dt>
    <dd class="dm:m-0">{detailDate(job.finishedAt)}</dd>
  </dl>
  {#if job.output}<pre
      class="dm:whitespace-pre-wrap dm:wrap-anywhere dm:text-xs">{JSON.stringify(
        job.output,
        null,
        2,
      )}</pre>{/if}
</Disclosure>
