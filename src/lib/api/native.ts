import { getIdentifier, getName, getTauriVersion, getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { downloadDir } from "@tauri-apps/api/path";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import type { AppInfo, JobEvent } from "$lib/model/types";

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
  const [name, version, identifier, tauriVersion] = await Promise.all([
    getName(),
    getVersion(),
    getIdentifier(),
    getTauriVersion(),
  ]);

  return { name, version, identifier, tauriVersion };
}

export function getSystemDownloadDirectory() {
  return downloadDir();
}

export function chooseDirectory(options: DirectorySelection) {
  return openDialog({
    directory: true,
    multiple: false,
    canCreateDirectories: options.canCreateDirectories,
    defaultPath: options.defaultPath,
    title: options.title,
  });
}

export function openExternalUrl(url: string) {
  return openUrl(url);
}

export async function downloadAndInstallAvailableUpdate(
  onProgress: (progress: AppUpdateProgress) => void,
) {
  const update = await check();

  if (!update) {
    return null;
  }

  let downloadedBytes = 0;
  let contentLength: number | undefined;

  await update.downloadAndInstall((event) => {
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
  });

  return update.version;
}

export function relaunchApp() {
  return relaunch();
}

export function listenToJobEvents(handler: (event: JobEvent) => void) {
  return listen<JobEvent>("dm-job-event", (event) => handler(event.payload));
}
