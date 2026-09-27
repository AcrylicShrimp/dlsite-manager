import { invoke as rawInvoke } from "@tauri-apps/api/core";
import type { AuditEvent } from "$lib/model/types";

type Failure = { clientId: string; command: string; category: string; repeatCount: number; outcome?: "succeeded" | "cancelled"; durationMs?: number };
const pending: Failure[] = [];
let dropped = 0;
let sending = false;
let reportedInWindow = 0;
let windowStart = 0;

// Fixed vocabulary: never retain messages, arguments, rejection objects, or DOM values.
const allowedCommands = new Set([
  "get_settings", "save_settings", "list_accounts", "save_account", "set_account_enabled",
  "remove_account", "list_products", "list_product_filter_facets", "get_product_detail",
  "set_product_custom_tags", "start_account_sync", "start_work_download", "preview_bulk_work_download",
  "start_bulk_work_download", "open_work_download", "delete_work_download", "mark_work_downloaded",
  "list_jobs", "cancel_job", "clear_finished_jobs", "list_audit_events", "get_audit_log_dir",
  "open_audit_log_dir", "submit_two_factor_code", "cancel_two_factor", "open_external_url",
  "native.dialog", "native.updater", "native.relaunch", "native.appInfo", "native.path",
  "native.listener", "frontend.error", "frontend.rejection",
]);

export function noteFailure(command: string, error?: unknown) { noteObservation(command, error); }
function noteObservation(command: string, error?: unknown, outcome?: "succeeded" | "cancelled", durationMs?: number) {
  const now = Date.now();
  if (now - windowStart >= 1000) { windowStart = now; reportedInWindow = 0; }
  if (++reportedInWindow > 10) { dropped++; return; }
  // A backend-correlated error already has a durable owner; don't log it twice.
  if (typeof error === "string" && /\[diagnostic:run-[0-9a-f-]{36}:op-[0-9a-f-]{36}\]$/.test(error)) return;
  command = allowedCommands.has(command) ? command : "frontend.error";
  const existing = pending.find((item) => item.command === command && item.outcome === outcome);
  if (existing) { existing.repeatCount = Math.min(10000, existing.repeatCount + 1); }
  else {
    if (pending.length >= 64) { pending.shift(); dropped++; }
    pending.push({ clientId: crypto.randomUUID(), command, category: command.startsWith("frontend.") ? "frontend" : "transport", repeatCount: 1, ...(outcome ? { outcome, durationMs } : {}) });
  }
  void flushReports();
}

async function flushReports() {
  if (sending) return;
  sending = true;
  try {
    while (pending.length) {
      const item = pending[0];
      // Snapshot, so a concurrent repeat cannot mutate the in-flight payload.
      const snapshot = { ...item };
      await rawInvoke("report_frontend_failure", { failure: snapshot });
      if (item.repeatCount > snapshot.repeatCount) {
        item.repeatCount -= snapshot.repeatCount;
        item.clientId = crypto.randomUUID();
      } else { const index = pending.indexOf(item); if (index >= 0) pending.splice(index, 1); }
    }
  } catch { /* Retain bounded safe reports; reporting never reports itself. */ }
  finally { sending = false; }
}

export async function diagnosticInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    const value = await rawInvoke<T>(command, args);
    void flushReports();
    return value;
  } catch (error) { noteFailure(command, error); throw error; }
}
export async function observeNative<T>(name: string, action: () => Promise<T>): Promise<T> {
  const started = Date.now();
  try {
    const value = await action();
    noteObservation(name, undefined, name === "native.dialog" && value === null ? "cancelled" : "succeeded", Date.now() - started);
    return value;
  } catch (error) { noteFailure(name, error); throw error; }
}
export function installDiagnosticHandlers() {
  const onError = () => noteFailure("frontend.error");
  const onRejection = () => noteFailure("frontend.rejection");
  window.addEventListener("error", onError);
  window.addEventListener("unhandledrejection", onRejection);
  return () => { window.removeEventListener("error", onError); window.removeEventListener("unhandledrejection", onRejection); };
}
export async function supportSummary(): Promise<string> {
  let backend: unknown;
  try { backend = await rawInvoke("diagnostic_summary"); }
  catch { backend = { state: "unavailable" }; }
  return JSON.stringify({ backend, frontend: { pending: pending.map(x => ({ ...x })), dropped } }, null, 2);
}
export const getDiagnosticSummary = () => rawInvoke<{ runId: string; health: { state: string; droppedRoutine: number; droppedCritical: number; ringEvicted: number } }>("diagnostic_summary");
export const diagnosticRuns = () => rawInvoke<{ runId: string; current: boolean }[]>("diagnostic_runs");
export const operationEvents = (operationId: string) => rawInvoke<AuditEvent[]>("diagnostic_operation", { operationId });
export const exportDiagnostics = async (runId?: string, operationId?: string) => { await flushReports(); return rawInvoke<string | null>("export_diagnostics", { scope: { runId: runId || null, operationId: operationId || null } }); };
