import type {
  AuditEvent,
  AuditOutcome,
  BulkFailedWork,
  BulkSucceededWork,
  JobSnapshot,
} from "$lib/model/types";
import { formatBytes, shortDate } from "$lib/utils/format";

export function upsertJob(currentJobs: JobSnapshot[], job: JobSnapshot) {
  const index = currentJobs.findIndex((item) => item.id === job.id);

  if (index === -1) {
    return [...currentJobs, job];
  }

  const next = currentJobs.slice();
  next[index] = job;
  return next;
}

export function isDownloadQueueJob(job: JobSnapshot) {
  return job.kind === "workDownload" || job.kind === "bulkWorkDownload";
}

export function auditOutcomeLabel(outcome: AuditOutcome) {
  switch (outcome) {
    case "queued":
      return "Queued";
    case "succeeded":
      return "Succeeded";
    case "failed":
      return "Failed";
    case "cancelled":
      return "Cancelled";
  }
}

export function auditDetail(event: AuditEvent) {
  return event.errorMessage ?? event.message;
}

export function isActiveJob(job: JobSnapshot) {
  return job.status === "queued" || job.status === "running" || job.status === "cancelling";
}

export function isTerminalJob(job: JobSnapshot) {
  return job.status === "succeeded" || job.status === "failed" || job.status === "cancelled";
}

export function jobAccountId(job: JobSnapshot) {
  const accountId = job.metadata.accountId;
  return typeof accountId === "string" ? accountId : null;
}

export function jobWorkId(job: JobSnapshot) {
  const workId = job.metadata.workId;
  return typeof workId === "string" ? workId : null;
}

export function jobLabel(job: JobSnapshot) {
  if (job.status === "queued") {
    return "Queued";
  }

  if (job.status === "cancelling") {
    return "Cancelling";
  }

  if (job.status === "failed") {
    return "Failed";
  }

  if (job.status === "cancelled") {
    return "Cancelled";
  }

  if (job.status === "succeeded") {
    if (job.kind === "workDownload") {
      return "Downloaded";
    }

    if (job.kind === "bulkWorkDownload") {
      const succeededCount = jobOutputNumber(job, "succeededCount");
      return typeof succeededCount === "number"
        ? `Downloaded ${succeededCount} works`
        : "Downloaded";
    }

    if (job.kind === "bulkWorkDownloadPreview") {
      const plannedCount = jobOutputNumber(job, "plannedCount");
      const failedCount = jobOutputNumber(job, "failedCount");

      if (
        typeof plannedCount === "number" &&
        typeof failedCount === "number" &&
        failedCount > 0
      ) {
        return `Planned ${plannedCount}, ${failedCount} failed`;
      }

      return typeof plannedCount === "number" ? `Planned ${plannedCount} works` : "Planned";
    }

    const cachedCount = jobOutputNumber(job, "cachedWorkCount");
    return typeof cachedCount === "number" ? `Synced ${cachedCount} works` : "Synced";
  }

  switch (job.phase) {
    case "loggingIn":
      return "Signing in";
    case "loadingCount":
      return "Checking library";
    case "loadingPurchases":
      return "Loading purchases";
    case "loadingWorks":
      return `Loading ${job.progress?.total ?? 0} works`;
    case "loadingProducts":
      return "Preparing downloads";
    case "bulkDownloading":
      return bulkDownloadJobProgressLabel(job);
    case "bulkPlanning":
      return bulkDownloadPlanningJobProgressLabel(job);
    case "committing":
      return "Saving cache";
    case "completed":
      return "Completing";
    case "resolvingDownload":
      return "Resolving download";
    case "probingDownload":
      return job.kind === "workDownload" ? downloadJobProgressLabel(job) : "Resolving download";
    case "downloading":
      return downloadJobProgressLabel(job);
    case "unpacking":
      return "Decompressing";
    case "finalizing":
      return "Finalizing";
    default:
      if (job.kind === "bulkWorkDownload") {
        return "Downloading results";
      }

      if (job.kind === "bulkWorkDownloadPreview") {
        return "Planning";
      }

      return job.kind === "workDownload" ? "Downloading" : "Syncing";
  }
}

export function activeJobDetail(job: JobSnapshot) {
  if (job.kind === "workDownload") {
    return activeWorkDownloadDetail(job);
  }

  if (job.kind === "bulkWorkDownload") {
    return activeBulkDownloadDetail(job);
  }

  return null;
}

export function activeWorkDownloadDetail(job: JobSnapshot) {
  if (job.status === "queued") {
    return "Waiting to start";
  }

  switch (job.phase) {
    case "loggingIn":
      return "Signing in";
    case "resolvingDownload":
      return "Resolving download files";
    case "probingDownload":
    case "downloading":
      return downloadProgressDetail(job);
    case "unpacking":
      return "Decompressing archive";
    case "finalizing":
      return "Moving files into the library";
    case "completed":
      return "Completing";
    default:
      return "Preparing download";
  }
}

export function activeBulkDownloadDetail(job: JobSnapshot) {
  if (job.status === "queued") {
    return "Waiting to start";
  }

  if (job.phase === "bulkDownloading" && job.progress?.unit === "items") {
    const current = job.progress.current ?? 0;
    const total = job.progress.total;

    return typeof total === "number" && total > 0
      ? `${current} of ${total} products processed`
      : "Processing products";
  }

  return "Preparing bulk download";
}

export function downloadQueueSubtitle(job: JobSnapshot) {
  if (job.kind === "workDownload") {
    const workId = jobWorkId(job) ?? jobOutputString(job, "workId");
    return workId ? `Single work ${workId}` : "Single work download";
  }

  const skippedDownloaded =
    jobOutputNumber(job, "skippedDownloadedCount") ?? metadataNumber(job, "skippedDownloadedCount");
  const skippedQueued =
    jobOutputNumber(job, "skippedQueuedCount") ?? metadataNumber(job, "skippedQueuedCount");
  const parts = [
    typeof skippedDownloaded === "number" ? `${skippedDownloaded} already downloaded` : null,
    typeof skippedQueued === "number" ? `${skippedQueued} already queued` : null,
  ].filter((part): part is string => part !== null);

  return parts.length > 0 ? parts.join(", ") : "Current Library filters";
}

export function downloadQueueKindLabel(job: JobSnapshot) {
  if (job.kind === "bulkWorkDownload") {
    return "Bulk";
  }

  return "Work";
}

export function downloadQueueTime(job: JobSnapshot) {
  return shortDate(job.finishedAt ?? job.startedAt ?? job.createdAt);
}

export function downloadQueueProgressPercent(job: JobSnapshot) {
  const current = job.progress?.current;
  const total = job.progress?.total;

  if (typeof current !== "number" || typeof total !== "number" || total <= 0) {
    return null;
  }

  return Math.min(100, Math.max(0, Math.floor((current * 100) / total)));
}

export function metadataNumber(job: JobSnapshot, key: string) {
  const value = job.metadata[key];
  return typeof value === "number" ? value : null;
}

export function jobOutputString(job: JobSnapshot, key: string) {
  const value = job.output?.[key];
  return typeof value === "string" ? value : null;
}

export function jobOutputBoolean(job: JobSnapshot, key: string) {
  const value = job.output?.[key];
  return typeof value === "boolean" ? value : false;
}

export function jobOutputNumber(job: JobSnapshot, key: string) {
  const value = job.output?.[key];
  return typeof value === "number" ? value : null;
}

export function bulkDownloadResult(job: JobSnapshot) {
  const source = job.output ?? recordValue(job.error?.details.bulkDownload);

  return {
    succeededWorks: parseBulkSucceededWorks(source?.succeededWorks),
    failedWorks: parseBulkFailedWorks(source?.failedWorks),
  };
}

export function parseBulkSucceededWorks(value: unknown): BulkSucceededWork[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value
    .map((item) => {
      const record = recordValue(item);

      if (!record || typeof record.workId !== "string") {
        return null;
      }

      return {
        workId: record.workId,
        localPath: typeof record.localPath === "string" ? record.localPath : null,
        fileCount: typeof record.fileCount === "number" ? record.fileCount : null,
        archiveExtracted:
          typeof record.archiveExtracted === "boolean" ? record.archiveExtracted : null,
      };
    })
    .filter((item): item is BulkSucceededWork => item !== null);
}

export function parseBulkFailedWorks(value: unknown): BulkFailedWork[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value
    .map((item) => {
      const record = recordValue(item);

      if (!record || typeof record.workId !== "string") {
        return null;
      }

      return {
        workId: record.workId,
        errorCode: typeof record.errorCode === "string" ? record.errorCode : null,
        errorMessage: typeof record.errorMessage === "string" ? record.errorMessage : null,
      };
    })
    .filter((item): item is BulkFailedWork => item !== null);
}

export function recordValue(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

export function downloadJobProgressLabel(job: JobSnapshot) {
  if (job.progress?.unit !== "bytes") {
    return "Downloading";
  }

  const current = job.progress.current ?? 0;
  const total = job.progress.total;

  if (typeof total === "number" && total > 0) {
    return `Downloading ${Math.min(100, Math.floor((current * 100) / total))}%`;
  }

  return "Downloading";
}

export function downloadProgressDetail(job: JobSnapshot) {
  if (job.progress?.unit !== "bytes") {
    return "Downloading files";
  }

  const current = job.progress.current ?? 0;
  const total = job.progress.total;

  if (typeof total === "number" && total > 0) {
    return `${formatBytes(current)} of ${formatBytes(total)}`;
  }

  return `${formatBytes(current)} downloaded`;
}

export function bulkDownloadJobProgressLabel(job: JobSnapshot) {
  if (job.progress?.unit !== "items") {
    return "Downloading results";
  }

  const current = job.progress.current ?? 0;
  const total = job.progress.total;

  if (typeof total === "number" && total > 0) {
    return `Downloading ${current}/${total}`;
  }

  return "Downloading results";
}

export function bulkDownloadPlanningJobProgressLabel(job: JobSnapshot) {
  if (job.progress?.unit !== "items") {
    return "Planning";
  }

  const current = job.progress.current ?? 0;
  const total = job.progress.total;

  if (typeof total === "number" && total > 0) {
    return `Planning ${current}/${total}`;
  }

  return "Planning";
}
