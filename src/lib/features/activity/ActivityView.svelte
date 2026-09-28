<script lang="ts">
  import JobDetails from "./JobDetails.svelte";
  import PageHeader from "$lib/components/workspace/PageHeader.svelte";
  import PageToolbar from "$lib/components/workspace/PageToolbar.svelte";
  import Button from "$lib/components/workspace/Button.svelte";
  import Choices from "$lib/components/workspace/ChoiceGroup.svelte";
  import Search from "$lib/components/workspace/SearchField.svelte";
  import Row from "$lib/components/workspace/ListRow.svelte";
  import Modal from "$lib/components/workspace/Modal.svelte";
  import Icon from "$lib/components/workspace/Icon.svelte";
  import {
    isActiveJob,
    isDownloadQueueJob,
    jobLabel,
    auditDetail,
    auditOutcomeLabel,
  } from "$lib/utils/jobs";
  import { shortDate } from "$lib/utils/format";
  import type { AuditEvent, JobSnapshot } from "$lib/model/types";
  import DiagnosticsPanel from "./DiagnosticsPanel.svelte";
  import type { DiagnosticTarget } from "$lib/utils/diagnostic-message";

  let {
    tab = $bindable("history"),
    onViewOperation,
    jobs = [],
    jobLoading = false,
    auditEvents = [],
    auditLoading = false,
    auditLogDir = "",
    getJobTitle,
    getJobDetail,
    onReloadJobs,
    onClearJobs,
    onCancelJob,
    onOpenAuditFolder,
    onReloadAudit,
  }: {
    tab?: string;
    onViewOperation?: (target: DiagnosticTarget) => void;
    jobs?: JobSnapshot[];
    jobLoading?: boolean;
    auditEvents?: AuditEvent[];
    auditLoading?: boolean;
    auditLogDir?: string;
    getJobTitle: (job: JobSnapshot) => string;
    getJobDetail: (job: JobSnapshot) => string;
    onReloadJobs: () => void;
    onClearJobs: () => void;
    onCancelJob: (job: JobSnapshot) => void;
    onOpenAuditFolder: () => void;
    onReloadAudit: () => void;
  } = $props();

  let filter = $state("all");
  let search = $state("");
  let selectedJobId = $state<string | null>(null);
  let selectedEvent = $state<AuditEvent | null>(null);
  let exporting = $state(false);
  let diagnosticOperation = $state("");
  const selectedJob = $derived(
    jobs.find((j) => j.id === selectedJobId) ?? null,
  );
  const history = $derived(
    jobs.filter((j) => !isActiveJob(j) || !isDownloadQueueJob(j)),
  );
  const visibleJobs = $derived(
    history.filter((j) => filter !== "errors" || j.status === "failed"),
  );
  const visibleEvents = $derived(
    auditEvents.filter(
      (e) =>
        (filter !== "errors" || e.level === "error") &&
        `${e.message} ${e.operation} ${e.errorMessage ?? ""}`
          .toLowerCase()
          .includes(search.toLowerCase()),
    ),
  );
  function closeDetail() {
    selectedJobId = null;
    selectedEvent = null;
  }
  function exportFor(operationId = "") {
    diagnosticOperation = operationId;
    exporting = true;
    closeDetail();
  }
</script>

<div class="activity-layout dm:min-w-0">
  <PageHeader title="Activity">
    <Button onclick={() => exportFor()}
      ><Icon name="downloads" />Export diagnostics</Button
    >
    {#snippet tabs()}
      <Choices
        variant="tabs"
        label="Activity view"
        value={tab}
        onchange={(v) => {
          tab = v;
          filter = "all";
          if (v === "logs") onReloadAudit();
        }}
        options={[
          { value: "history", label: "Work history", count: history.length },
          {
            value: "logs",
            label: "Application logs",
            count: auditEvents.length,
          },
        ]}
      />
    {/snippet}
    {#snippet toolbar()}
      <PageToolbar
        label="Activity tools"
        onReload={tab === "history" ? onReloadJobs : onReloadAudit}
        reloadDisabled={tab === "history" ? jobLoading : auditLoading}
      >
        {#if tab === "logs"}<div class="dm:flex dm:min-w-0 dm:basis-48 dm:grow">
            <Search
              bind:value={search}
              label="Search logs"
              placeholder="Search logs…"
              clearable
            />
          </div>{/if}
        <Choices
          label="Activity filter"
          value={filter}
          onchange={(v) => (filter = v)}
          options={[
            { value: "all", label: "All" },
            { value: "errors", label: "Errors" },
          ]}
        />
        {#snippet actions()}
          {#if tab === "history"}<Button
              disabled={jobLoading || !jobs.some((j) => !isActiveJob(j))}
              onclick={onClearJobs}>Clear finished</Button
            >{:else}<Button disabled={!auditLogDir} onclick={onOpenAuditFolder}
              ><Icon name="folder" />Open log folder</Button
            >{/if}
        {/snippet}
      </PageToolbar>
    {/snippet}
  </PageHeader>
  {#if tab === "history"}
    {#if jobLoading}<p
        role="status"
        class="dm:py-10 dm:text-center dm:text-draft-dim"
      >
        Loading…
      </p>{:else if !visibleJobs.length}<p
        class="dm:py-10 dm:text-center dm:text-draft-dim"
      >
        No matching work
      </p>{/if}
    <div aria-label="Work history">
      {#each visibleJobs as job (job.id)}
        <Row onclick={() => (selectedJobId = job.id)}>
          {#snippet leading()}<span
              class={`dm:grid dm:size-8 dm:shrink-0 dm:place-items-center dm:rounded-full ${job.status === "failed" ? "dm:bg-draft-error-bg dm:text-draft-error" : "dm:bg-draft-selected dm:text-draft-accent"}`}
              ><Icon
                name={job.status === "failed"
                  ? "info"
                  : job.status === "succeeded"
                    ? "check"
                    : "refresh"}
              /></span
            >{/snippet}
          <strong>{getJobTitle(job)}</strong><span
            class="dm:text-sm dm:text-draft-dim">{getJobDetail(job)}</span
          >
          {#snippet trailing()}<span
              class={job.status === "failed" ? "dm:text-draft-error" : ""}
              >{jobLabel(job)}</span
            ><time class="dm:text-xs"
              >{shortDate(
                job.finishedAt ?? job.startedAt ?? job.createdAt,
              )}</time
            >{/snippet}
        </Row>
      {/each}
    </div>
  {:else}
    {#if auditLoading}<p
        role="status"
        class="dm:py-10 dm:text-center dm:text-draft-dim"
      >
        Loading…
      </p>{:else if !visibleEvents.length}<p
        class="dm:py-10 dm:text-center dm:text-draft-dim"
      >
        No matching events
      </p>{/if}
    <div aria-label="Application logs">
      {#each visibleEvents as event, i (`${event.at}-${i}`)}
        <Row
          onclick={() => {
            if (onViewOperation && event.runId && event.operationId) {
              onViewOperation({
                runId: event.runId,
                operationId: event.operationId,
                message: auditDetail(event),
              });
            } else selectedEvent = event;
          }}
        >
          {#snippet leading()}<span
              class={`dm:w-12 dm:shrink-0 dm:text-xs dm:uppercase ${event.level === "error" ? "dm:text-draft-error" : "dm:text-draft-dim"}`}
              >{event.level}</span
            >{/snippet}
          <strong>{auditDetail(event)}</strong><span
            class="dm:text-sm dm:text-draft-dim"
            >{event.operation} · {auditOutcomeLabel(event.outcome)}</span
          >
          {#snippet trailing()}<time class="dm:text-xs"
              >{shortDate(event.at)}</time
            >{/snippet}
        </Row>
      {/each}
    </div>
  {/if}
</div>
{#if selectedJob || selectedEvent}
  <Modal title="Activity details" onClose={closeDetail}>
    {#if selectedJob}<JobDetails
        job={selectedJob}
        title={getJobTitle(selectedJob)}
        detail={getJobDetail(selectedJob)}
      />
    {:else if selectedEvent}<h3 class="dm:mt-0 dm:wrap-anywhere">
        {selectedEvent.operation}
      </h3>
      <p>{auditOutcomeLabel(selectedEvent.outcome)}</p>
      <p class="dm:wrap-anywhere">{auditDetail(selectedEvent)}</p>
      <details>
        <summary class="dm:draft-focus-row dm:p-3">Operation details</summary>
        <pre
          class="dm:whitespace-pre-wrap dm:wrap-anywhere dm:text-xs">{JSON.stringify(
            selectedEvent.details,
            null,
            2,
          )}</pre>
      </details>{/if}
    {#snippet actions()}
      {#if selectedJob && isActiveJob(selectedJob)}<Button
          disabled={!selectedJob.cancellable ||
            selectedJob.status === "cancelling"}
          onclick={() => selectedJob && onCancelJob(selectedJob)}
          >Cancel work</Button
        >{/if}
      <Button
        onclick={() =>
          exportFor(
            selectedEvent?.operationId ??
              auditEvents.find((e) => e.details?.jobId === selectedJob?.id)
                ?.operationId ??
              "",
          )}>Export diagnostics</Button
      >
    {/snippet}
  </Modal>
{/if}
{#if exporting}<Modal
    title="Export diagnostics"
    wide
    onClose={() => (exporting = false)}
    ><DiagnosticsPanel
      events={auditEvents}
      initialOperationId={diagnosticOperation}
    /></Modal
  >{/if}
