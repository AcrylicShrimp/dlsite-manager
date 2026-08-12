import type { JobSnapshot } from "$lib/model/types";

const createdAt = "2026-08-12T05:12:00Z";

export const queuedDownloadJob: JobSnapshot = {
  id: "job-queued",
  kind: "workDownload",
  title: "Download work",
  status: "queued",
  phase: null,
  progress: null,
  metadata: { workId: "RJ01553954" },
  output: null,
  error: null,
  cancellable: true,
  createdAt,
  startedAt: null,
  finishedAt: null,
};

export const runningDownloadJob: JobSnapshot = {
  id: "job-running",
  kind: "workDownload",
  title: "Download work",
  status: "running",
  phase: "downloading",
  progress: { current: 322_000_000, total: 482_000_000, unit: "bytes" },
  metadata: { workId: "RJ01234567" },
  output: null,
  error: null,
  cancellable: true,
  createdAt,
  startedAt: "2026-08-12T05:14:00Z",
  finishedAt: null,
};

export const cancellingDownloadJob: JobSnapshot = {
  id: "job-cancelling",
  kind: "bulkWorkDownload",
  title: "Bulk download",
  status: "cancelling",
  phase: "bulkDownloading",
  progress: { current: 12, total: 30, unit: "items" },
  metadata: { reservedCount: 30, skippedDownloadedCount: 8 },
  output: null,
  error: null,
  cancellable: false,
  createdAt,
  startedAt: "2026-08-12T05:13:00Z",
  finishedAt: null,
};

export const failedDownloadJob: JobSnapshot = {
  id: "job-failed",
  kind: "workDownload",
  title: "Download work",
  status: "failed",
  phase: "downloading",
  progress: { current: 176_000_000, total: 482_000_000, unit: "bytes" },
  metadata: { workId: "RJ09999999" },
  output: null,
  error: {
    code: "network",
    message: "The download stream ended before the archive was complete.",
    details: {},
  },
  cancellable: false,
  createdAt,
  startedAt: "2026-08-12T05:14:00Z",
  finishedAt: "2026-08-12T05:17:00Z",
};
