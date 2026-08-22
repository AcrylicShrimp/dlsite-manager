import { invoke } from "@tauri-apps/api/core";
import type {
  Account,
  AccountRemovalReport,
  AppSettings,
  AuditEvent,
  BulkWorkDownloadPreview,
  JobSnapshot,
  ProductCustomTag,
  ProductDetail,
  ProductDownload,
  ProductFilterFacets,
  ProductListPage,
  StartJobResponse,
} from "$lib/model/types";

export type SaveSettingsRequest = {
  libraryRoot: string | null;
  downloadRoot: string | null;
};

export type SaveAccountRequest = {
  id: string | null;
  label: string;
  loginName: string | null;
  password: string | null;
};

export type ProductListRequest = {
  search: string | null;
  accountIds: string[];
  typeGroups: string[];
  ageCategories: string[];
  sourceGroups: string[];
  makerNames: string[];
  customTagNames: string[];
  excludedCustomTagNames: string[];
  sort: string;
  limit: number;
  offset: number;
};

export type BulkWorkDownloadRequest = Omit<ProductListRequest, "limit" | "offset"> & {
  unpackPolicy: "keepArchives" | "unpackWhenRecognized";
  skipDownloaded: boolean;
};

export type StartWorkDownloadRequest = {
  workId: string;
  accountId: string | null;
  password: string | null;
  unpackPolicy: "keepArchives" | "unpackWhenRecognized";
  replaceExisting: boolean;
};

export type CancelJobResult = {
  outcome: "requested" | "alreadyFinished";
  snapshot: JobSnapshot;
};

export type ClearFinishedJobsResult = {
  removedCount: number;
};

export function getSettings() {
  return invoke<AppSettings>("get_settings");
}

export function saveSettings(settings: SaveSettingsRequest) {
  return invoke<AppSettings>("save_settings", { settings });
}

export function listAccounts() {
  return invoke<Account[]>("list_accounts");
}

export function saveAccount(request: SaveAccountRequest) {
  return invoke<Account>("save_account", { request });
}

export function setAccountEnabled(accountId: string, enabled: boolean) {
  return invoke<void>("set_account_enabled", { request: { accountId, enabled } });
}

export function removeAccount(accountId: string) {
  return invoke<AccountRemovalReport>("remove_account", { request: { accountId } });
}

export function listProducts(request: ProductListRequest) {
  return invoke<ProductListPage>("list_products", { request });
}

export function listProductFilterFacets(request: ProductListRequest) {
  return invoke<ProductFilterFacets>("list_product_filter_facets", { request });
}

export function getProductDetail(workId: string) {
  return invoke<ProductDetail>("get_product_detail", { request: { workId } });
}

export function setProductCustomTags(workId: string, tags: string[]) {
  return invoke<ProductCustomTag[]>("set_product_custom_tags", {
    request: { workId, tags },
  });
}

export function startAccountSync(accountId: string, password: string | null) {
  return invoke<StartJobResponse>("start_account_sync", {
    request: { accountId, password },
  });
}

export function startWorkDownload(request: StartWorkDownloadRequest) {
  return invoke<StartJobResponse>("start_work_download", { request });
}

export function previewBulkWorkDownload(request: BulkWorkDownloadRequest) {
  return invoke<BulkWorkDownloadPreview>("preview_bulk_work_download", { request });
}

export function startBulkWorkDownload(request: BulkWorkDownloadRequest) {
  return invoke<StartJobResponse>("start_bulk_work_download", { request });
}

export function openWorkDownload(workId: string) {
  return invoke<void>("open_work_download", { request: { workId } });
}

export function deleteWorkDownload(workId: string) {
  return invoke<ProductDownload>("delete_work_download", { request: { workId } });
}

export function markWorkDownloaded(workId: string, localPath: string) {
  return invoke<ProductDownload>("mark_work_downloaded", { request: { workId, localPath } });
}

export function listJobs() {
  return invoke<JobSnapshot[]>("list_jobs");
}

export function cancelJob(jobId: string) {
  return invoke<CancelJobResult>("cancel_job", { request: { jobId } });
}

export function clearFinishedJobs() {
  return invoke<ClearFinishedJobsResult>("clear_finished_jobs");
}

export function listAuditEvents(limit: number) {
  return invoke<AuditEvent[]>("list_audit_events", { request: { limit } });
}

export function getAuditLogDir() {
  return invoke<{ path: string }>("get_audit_log_dir");
}

export function openAuditLogDir() {
  return invoke<void>("open_audit_log_dir");
}

export function submitTwoFactorCode(requestId: string, code: string) {
  return invoke<void>("submit_two_factor_code", { request: { requestId, code } });
}

export function cancelTwoFactor(requestId: string) {
  return invoke<void>("cancel_two_factor", { request: { requestId } });
}
