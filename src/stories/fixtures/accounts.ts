import type { Account, JobSnapshot } from "$lib/model/types";

export const primaryAccount: Account = {
  id: "primary",
  label: "Primary DLsite account",
  loginName: "primary@example.test",
  hasCredential: true,
  enabled: true,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-08-12T05:00:00Z",
  lastLoginAt: "2026-08-12T04:58:00Z",
  lastSyncAt: "2026-08-12T05:00:00Z",
};

export const disabledAccount: Account = {
  id: "archive",
  label: "Archive account with a deliberately long source name",
  loginName: null,
  hasCredential: false,
  enabled: false,
  createdAt: "2026-02-01T00:00:00Z",
  updatedAt: "2026-07-21T02:00:00Z",
  lastLoginAt: null,
  lastSyncAt: null,
};

export const syncingAccount: Account = {
  id: "syncing",
  label: "Secondary purchases",
  loginName: "secondary@example.test",
  hasCredential: true,
  enabled: true,
  createdAt: "2026-03-01T00:00:00Z",
  updatedAt: "2026-08-12T05:05:00Z",
  lastLoginAt: "2026-08-12T05:04:00Z",
  lastSyncAt: "2026-08-01T01:00:00Z",
};

export const accountSyncJob: JobSnapshot = {
  id: "job-account-sync",
  kind: "accountSync",
  title: "Sync account",
  status: "running",
  phase: "loadingWorks",
  progress: { current: 42, total: 120, unit: "items" },
  metadata: { accountId: syncingAccount.id },
  output: null,
  error: null,
  cancellable: true,
  createdAt: "2026-08-12T05:06:00Z",
  startedAt: "2026-08-12T05:06:00Z",
  finishedAt: null,
};
