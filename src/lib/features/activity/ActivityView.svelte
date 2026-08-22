<script lang="ts">
  import UiButton from "$lib/components/ui/Button.svelte";
  import type { AuditEvent, JobSnapshot } from "$lib/model/types";
  import AuditLogList from "./AuditLogList.svelte";
  import JobList from "./JobList.svelte";

  let {
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
</script>

<div class="activity-layout">
  <section class="activity-panel" aria-label="Jobs">
    <div class="panel-title">
      <h2>Jobs</h2>
      <div class="panel-actions">
        <UiButton variant="secondary" size="small" disabled={jobLoading} onclick={onReloadJobs}>
          Reload
        </UiButton>
        <UiButton size="small" disabled={jobLoading} onclick={onClearJobs}>Clear</UiButton>
      </div>
    </div>
    <JobList
      {jobs}
      loading={jobLoading}
      getTitle={getJobTitle}
      getDetail={getJobDetail}
      onCancel={onCancelJob}
    />
  </section>

  <section class="activity-panel" aria-label="Audit log">
    <div class="panel-title">
      <div>
        <h2>Audit log</h2>
        <p>{auditLogDir || "App log directory"}</p>
      </div>
      <div class="panel-actions">
        <UiButton
          variant="secondary"
          size="small"
          disabled={!auditLogDir}
          onclick={onOpenAuditFolder}
        >
          Open Folder
        </UiButton>
        <UiButton variant="secondary" size="small" disabled={auditLoading} onclick={onReloadAudit}>
          Reload
        </UiButton>
      </div>
    </div>
    <AuditLogList events={auditEvents} loading={auditLoading} />
  </section>
</div>

<style>
  .activity-layout {
    display: grid;
    flex: 1 1 auto;
    grid-template-rows: minmax(120px, 0.42fr) minmax(0, 1fr);
    gap: 18px;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .activity-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 18px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    box-shadow: 0 16px 40px rgb(0 0 0 / 18%);
    overflow: hidden;
  }

  .panel-title,
  .panel-actions {
    display: flex;
    align-items: center;
  }

  .panel-title {
    flex: 0 0 auto;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 14px;
  }

  .panel-title > div {
    min-width: 0;
  }

  .panel-actions {
    gap: 8px;
  }

  h2 {
    margin: 0;
    color: var(--text-strong);
    font-size: 17px;
    font-weight: 700;
  }

  p {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.35;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 720px) {
    .activity-layout {
      grid-template-rows: auto auto;
      overflow: auto;
    }

    .panel-title,
    .panel-actions {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
