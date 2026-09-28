<script lang="ts">
  import type { AuditEvent, JobSnapshot } from "$lib/model/types";
  import { jobDetail } from "../fixtures/redesign";
  import Row from "./DraftListRow.svelte";
  import Icon from "./DraftIcon.svelte";
  let {
    jobs = [],
    events = [],
    mode,
    onjob,
    onevent,
  }: {
    jobs?: JobSnapshot[];
    events?: AuditEvent[];
    mode: "history" | "logs";
    onjob: (job: JobSnapshot) => void;
    onevent: (event: AuditEvent) => void;
  } = $props();
</script>

<div
  class="dm:mt-2"
  aria-label={mode === "history" ? "Work history" : "Application logs"}
>
  {#if mode === "history"}
    {#each jobs as job}
      <Row class="history-row" onclick={() => onjob(job)}>
        {#snippet leading()}<span
            class={`dm:grid dm:size-8 dm:place-items-center dm:rounded-full ${job.status === "failed" ? "dm:bg-draft-error-bg dm:text-draft-error" : "dm:bg-draft-selected dm:text-draft-accent"}`}
            ><Icon
              name={job.status === "failed"
                ? "info"
                : job.status === "succeeded"
                  ? "check"
                  : "refresh"}
            /></span
          >{/snippet}
        <strong class="dm:text-base dm:font-semibold">{job.title}</strong>
        <span class="dm:text-xs dm:text-draft-dim">{jobDetail(job)}</span>
        {#snippet trailing()}<span
            class={job.status === "failed"
              ? "dm:text-draft-error"
              : "dm:text-draft-ink"}
            >{job.status === "succeeded"
              ? "Completed"
              : job.status === "failed"
                ? "Failed"
                : job.status === "running"
                  ? "Syncing"
                  : "Cancelled"}</span
          ><small class="dm:text-xs">Today</small>{/snippet}
      </Row>
    {/each}
  {:else}
    {#each events as event}
      <Row class="log-row" onclick={() => onevent(event)}>
        {#snippet leading()}<span class="dm:flex dm:w-12 dm:flex-col dm:gap-1"
            ><time class="dm:text-xs dm:tabular-nums dm:text-draft-dim"
              >{event.at.slice(11, 16)}</time
            ><span
              class={`dm:text-xs dm:uppercase ${event.level === "error" ? "dm:text-draft-error" : "dm:text-draft-dim"}`}
              >{event.level}</span
            ></span
          >{/snippet}
        <strong class="dm:text-base dm:font-semibold">{event.message}</strong
        ><span class="dm:text-xs dm:text-draft-dim">{event.operation}</span>
      </Row>
    {/each}
  {/if}
</div>
