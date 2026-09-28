import { getIdentifier, getName, getTauriVersion, getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { downloadDir } from "@tauri-apps/api/path";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { diagnosticInvoke, observeNative } from "./diagnostics";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import type { AppInfo, JobEvent, TwoFactorClosed, TwoFactorRequest } from "$lib/model/types";

export type AppUpdateProgress = {
  phase: "downloading" | "installing";
  version: string;
  downloadedBytes: number;
  contentLength?: number;
};

export type DirectorySelection = {
  title: string;
  defaultPath?: string;
  canCreateDirectories: boolean;
};

export async function getAppInfo(): Promise<AppInfo> {
  const [name, version, identifier, tauriVersion] = await observeNative("native.appInfo", () => Promise.all([
    getName(),
    getVersion(),
    getIdentifier(),
    getTauriVersion(),
  ]));

  return { name, version, identifier, tauriVersion };
}

export function getSystemDownloadDirectory() {
  return observeNative("native.path", () => downloadDir());
}

export function chooseDirectory(options: DirectorySelection) {
  return observeNative("native.dialog", () => openDialog({
    directory: true,
    multiple: false,
    canCreateDirectories: options.canCreateDirectories,
    defaultPath: options.defaultPath,
    title: options.title,
  }));
}

export function openExternalUrl(url: string) {
  return diagnosticInvoke<void>("open_external_url", { url });
}

export async function downloadAndInstallAvailableUpdate(
  onProgress: (progress: AppUpdateProgress) => void,
) {
  // GitHub release asset routes can return HTTP 500 for application/json Accept.
  // The updater still parses and validates the downloaded release metadata.
  const update = await observeNative("native.updater", () => check({ headers: { Accept: "*/*" } }));

  if (!update) {
    return null;
  }

  let downloadedBytes = 0;
  let contentLength: number | undefined;

  await observeNative("native.updater", () => update.downloadAndInstall((event) => {
    if (event.event === "Started") {
      downloadedBytes = 0;
      contentLength = event.data.contentLength;
      onProgress({
        phase: "downloading",
        version: update.version,
        downloadedBytes,
        contentLength,
      });
    } else if (event.event === "Progress") {
      downloadedBytes += event.data.chunkLength;
      onProgress({
        phase: "downloading",
        version: update.version,
        downloadedBytes,
        contentLength,
      });
    } else {
      onProgress({
        phase: "installing",
        version: update.version,
        downloadedBytes,
        contentLength,
      });
    }
  }));

  return update.version;
}

export function relaunchApp() {
  return observeNative("native.relaunch", () => relaunch());
}

export function listenToJobEvents(handler: (event: JobEvent) => void) {
  return observeNative("native.listener", () => listen<JobEvent>("dm-job-event", (event) => handler(event.payload)));
}

export function listenToTwoFactorRequests(handler: (request: TwoFactorRequest) => void) {
  return observeNative("native.listener", () => listen<TwoFactorRequest>("dm-two-factor-request", (event) => handler(event.payload)));
}

export function listenToTwoFactorClosures(handler: (closed: TwoFactorClosed) => void) {
  return observeNative("native.listener", () => listen<TwoFactorClosed>("dm-two-factor-closed", (event) => handler(event.payload)));
}
