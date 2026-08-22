<script lang="ts">
  import ActivityView from "$lib/features/activity/ActivityView.svelte";
  import type { AuditEvent, JobSnapshot } from "$lib/model/types";
  import { accountSyncJob } from "../fixtures/accounts";
  import { failedDownloadJob } from "../fixtures/jobs";

  let {
    viewState = "populated",
    withAuditDirectory = true,
  }: {
    viewState?: "populated" | "loading" | "empty";
    withAuditDirectory?: boolean;
  } = $props();

  const completedJob: JobSnapshot = {
    ...accountSyncJob,
    id: "job-completed-sync",
    status: "succeeded",
    phase: "completed",
    progress: { current: 120, total: 120, unit: "items" },
    output: { cachedWorkCount: 120 },
    cancellable: false,
    finishedAt: "2026-08-12T05:10:00Z",
  };

  const jobs = $derived(viewState === "populated" ? [accountSyncJob, failedDownloadJob, completedJob] : []);
  const events = $derived<AuditEvent[]>(viewState === "populated" ? [
    {
      at: "2026-08-12T05:17:00Z", level: "error", operation: "work.download",
      outcome: "failed", message: "Download failed", errorCode: "network",
      errorMessage: "The download stream ended before the archive was complete.", details: {},
    },
    {
      at: "2026-08-12T05:10:00Z", level: "info", operation: "account.sync",
      outcome: "succeeded", message: "Cached 120 works", errorCode: null, errorMessage: null, details: {},
    },
    {
      at: "2026-08-12T04:42:00Z", level: "warn", operation: "bulk.download",
      outcome: "cancelled", message: "Cancelled after 12 products", errorCode: null, errorMessage: null, details: {},
    },
  ] : []);

  let lastAction = $state("No action yet");
</script>

<main class="story-surface">
  <ActivityView
    {jobs}
    jobLoading={viewState === "loading"}
    auditEvents={events}
    auditLoading={viewState === "loading"}
    auditLogDir={withAuditDirectory ? "/Users/example/Library/Logs" : ""}
    getJobTitle={(job) => job.kind === "accountSync" ? "Primary DLsite account" : "RJ09999999"}
    getJobDetail={(job) => job.error?.message ?? (job.status === "running" ? "42 of 120 works loaded" : "Finished successfully")}
    onReloadJobs={() => (lastAction = "Reload jobs")}
    onClearJobs={() => (lastAction = "Clear jobs")}
    onCancelJob={(job) => (lastAction = `Cancel ${job.id}`)}
    onOpenAuditFolder={() => (lastAction = "Open audit folder")}
    onReloadAudit={() => (lastAction = "Reload audit")}
  />
  <p aria-live="polite">{lastAction}</p>
</main>

<style>
  .story-surface {
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    gap: 12px;
    width: min(1120px, calc(100vw - 48px));
    height: calc(100vh - 48px);
    margin: 24px auto;
  }

  p { margin: 0; color: var(--text-subtle); font-size: 12px; text-align: right; }

  @media (max-width: 720px) {
    .story-surface { width: calc(100vw - 24px); height: auto; min-height: calc(100vh - 24px); margin: 12px auto; }
  }
</style>
