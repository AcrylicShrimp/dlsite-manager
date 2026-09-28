<script lang="ts">
  import PageHeader from "$lib/components/workspace/PageHeader.svelte";
  import type { JobSnapshot } from "$lib/model/types";
  import Button from "$lib/components/workspace/Button.svelte";
  import Choices from "$lib/components/workspace/ChoiceGroup.svelte";
  import Row from "./DownloadQueueRow.svelte";
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
  let filter = $state("all");
  const visible = $derived(
    jobs.filter(
      (j) =>
        filter === "all" ||
        (filter === "queued" ? j.status === "queued" : j.status !== "queued"),
    ),
  );
</script>

<section aria-label="Downloads">
  <PageHeader title="Downloads"
    ><Button variant="text" disabled={loading} onclick={onReload}>Reload</Button
    ></PageHeader
  >
  <div
    class="dm:mb-4 dm:flex dm:flex-wrap dm:items-center dm:justify-between dm:gap-3"
  >
    <Choices
      label="Download status"
      value={filter}
      onchange={(v) => (filter = v)}
      options={[
        { value: "all", label: "All", count: jobs.length },
        { value: "running", label: "In progress", count: runningCount },
        { value: "queued", label: "Queued", count: queuedCount },
      ]}
    />
  </div>
  {#if loading}<p
      role="status"
      class="dm:py-10 dm:text-center dm:text-draft-dim"
    >
      Loading…
    </p>{:else if !visible.length}<p
      class="dm:py-10 dm:text-center dm:text-draft-dim"
    >
      No active downloads
    </p>{:else}
    <div aria-label="Download jobs">
      {#each visible as job (job.id)}<Row
          {job}
          title={getTitle(job)}
          detail={getDetail(job)}
          {onCancel}
        />{/each}
    </div>
  {/if}
</section>
